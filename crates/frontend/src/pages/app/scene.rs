pub mod camera;

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Deref;

use awsm_renderer::bounds::Aabb;
use awsm_renderer::core::command::color::Color;
use awsm_renderer::core::renderer;
use awsm_renderer::core::texture::TextureFormat;
use awsm_renderer::gltf::data::GltfData;
use awsm_renderer::gltf::loader::GltfLoader;
use awsm_renderer::lights::Light;
use awsm_renderer::mesh::MeshKey;
use awsm_renderer::pipeline::RenderPipelineKey;
use awsm_renderer::shaders::fragment::cache_key::ShaderCacheKeyFragment;
use awsm_renderer::shaders::fragment::entry::debug_normals::ShaderCacheKeyFragmentDebugNormals;
use awsm_renderer::shaders::vertex::entry::mesh::ShaderTemplateVertexMesh;
use awsm_renderer::{AwsmRenderer, AwsmRendererBuilder};
use awsm_web::dom::resize::ResizeObserver;
use camera::{Camera, CameraId};
use glam::Vec3;
use gloo_events::EventListener;
use serde::de;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::models::collections::GltfId;
use crate::pages::app::sidebar::current_model_signal;
use crate::pages::app::sidebar::material::FragmentShaderKind;
use crate::prelude::*;

use super::canvas;
use super::context::AppContext;

pub struct AppScene {
    pub ctx: AppContext,
    pub renderer: futures::lock::Mutex<AwsmRenderer>,
    pub gltf_loader: Mutex<HashMap<GltfId, GltfLoader>>,
    pub camera: Mutex<Option<Camera>>,
    pub resize_observer: Mutex<Option<ResizeObserver>>,
    pub request_animation_frame: Mutex<Option<gloo_render::AnimationFrame>>,
    pub last_request_animation_frame: Cell<Option<f64>>,
    pub event_listeners: Mutex<Vec<EventListener>>,
    last_size: Cell<(f64, f64)>,
    last_shader_kind: Cell<Option<FragmentShaderKind>>,
    old_shader_kind_material: Mutex<HashMap<(MeshKey, FragmentShaderKind), ShaderCacheKeyFragment>>,
}

impl AppScene {
    pub fn new(ctx: AppContext, renderer: AwsmRenderer) -> Arc<Self> {
        let canvas = renderer.gpu.canvas();

        let state = Arc::new(Self {
            ctx,
            renderer: futures::lock::Mutex::new(renderer),
            gltf_loader: Mutex::new(HashMap::new()),
            camera: Mutex::new(None),
            resize_observer: Mutex::new(None),
            request_animation_frame: Mutex::new(None),
            last_request_animation_frame: Cell::new(None),
            event_listeners: Mutex::new(Vec::new()),
            last_size: Cell::new((0.0, 0.0)),
            last_shader_kind: Cell::new(None),
            old_shader_kind_material: Mutex::new(HashMap::new()),
        });

        let resize_observer = ResizeObserver::new(
            clone!(canvas, state => move |entries| {
                if let Some(entry) = entries.get(0) {
                    let width = entry.content_box_sizes[0].inline_size;
                    let height = entry.content_box_sizes[0].block_size;
                    canvas.set_width(width);
                    canvas.set_height(height);

                    state.on_viewport_change();
                }
            }),
            None,
        );

        resize_observer.observe(&canvas);

        *state.resize_observer.lock().unwrap() = Some(resize_observer);

        let event_listeners = vec![
            EventListener::new(
                &canvas,
                "pointerdown",
                clone!(state => move |event| {
                    if let Some(camera) = state.camera.lock().unwrap().as_mut() {
                        camera.on_pointer_down();
                    }
                }),
            ),
            EventListener::new(
                &web_sys::window().unwrap(),
                "pointermove",
                clone!(state => move |event| {
                    if let Some(camera) = state.camera.lock().unwrap().as_mut() {
                        let event = event.unchecked_ref::<web_sys::PointerEvent>();
                        camera.on_pointer_move(event.movement_x(), event.movement_y());
                    }
                }),
            ),
            EventListener::new(
                &web_sys::window().unwrap(),
                "pointerup",
                clone!(state => move |event| {
                    if let Some(camera) = state.camera.lock().unwrap().as_mut() {
                        camera.on_pointer_up();
                    }
                }),
            ),
            EventListener::new(
                &canvas,
                "wheel",
                clone!(state => move |event| {
                    if let Some(camera) = state.camera.lock().unwrap().as_mut() {
                        let event = event.unchecked_ref::<web_sys::WheelEvent>();
                        camera.on_wheel(event.delta_y());
                    }
                }),
            ),
        ];

        *state.event_listeners.lock().unwrap() = event_listeners;

        spawn_local(clone!(state => async move {
            state.ctx.camera_id.signal().for_each(clone!(state => move |_| clone!(state => async move {
                state.on_viewport_change();
            }))).await;
        }));

        spawn_local(clone!(state => async move {
            state.ctx.shader.signal().for_each(clone!(state => move |_| clone!(state => async move {
                state.on_shader_change();
            }))).await;
        }));

        spawn_local(clone!(state => async move {
            state.ctx.post_processing.signal().for_each(clone!(state => move |_| clone!(state => async move {
                state.on_post_processing_change();
            }))).await;
        }));

        state
    }

    fn on_viewport_change(self: &Arc<Self>) {
        let state = self;

        spawn_local(clone!(state => async move {
            let last_size = state.last_size.get();

            {
                let renderer = state.renderer.lock().await;
                let (canvas_width, canvas_height) = renderer.gpu.canvas_size();
                if (canvas_width, canvas_height) == last_size {
                    return;
                }
                state.last_size.set((canvas_width, canvas_height));
            }
            if let Err(err) = state.setup_viewport().await {
                tracing::error!("Failed to setup scene after canvas resize: {:?}", err);
            }

            if let Err(err) = state.render().await {
                tracing::error!("Failed to render after canvas resize: {:?}", err);
            }
        }));
    }

    fn on_shader_change(self: &Arc<Self>) {
        let state = self;

        spawn_local(clone!(state => async move {
            let shader_kind = state.ctx.shader.get();
            let last_shader_kind = state.last_shader_kind.get();

            if Some(shader_kind) == last_shader_kind {
                return;
            }

            if let Err(err) = state.setup_shader().await {
                tracing::error!("Failed to change shader: {:?}", err);
            }

            if let Err(err) = state.render().await {
                tracing::error!("Failed to render after shader change: {:?}", err);
            }
        }));
    }

    fn on_post_processing_change(self: &Arc<Self>) {
        let state = self;

        spawn_local(clone!(state => async move {
            let mut renderer = state.renderer.lock().await;
            state.stop_animation_loop();
            renderer.post_process.settings = state.ctx.post_processing.clone().into();
            if let Err(err) = renderer.post_process_init().await {
                tracing::error!("Failed to initialize post process: {:?}", err);
            }
            if let Err(err) = renderer.post_process_update_view() {
                tracing::error!("Failed to update post process view: {:?}", err);
            }
            state.start_animation_loop();
        }));
    }

    pub async fn clear(self: &Arc<Self>) {
        let state = self;

        let mut renderer = state.renderer.lock().await;

        state.stop_animation_loop();
        if let Err(err) = renderer.remove_all().await {
            tracing::error!("Failed to clear renderer: {:?}", err);
        }
        renderer.render();
    }

    pub async fn load(self: &Arc<Self>, gltf_id: GltfId) -> Result<GltfLoader> {
        let state = self;

        if let Some(loader) = state.gltf_loader.lock().unwrap().get(&gltf_id).cloned() {
            return Ok(loader);
        }

        let url = format!("{}/{}", CONFIG.gltf_url, gltf_id.filepath());

        let loader = GltfLoader::load(&url, None).await?;

        state
            .gltf_loader
            .lock()
            .unwrap()
            .insert(gltf_id, loader.clone());

        Ok(loader)
    }

    pub async fn upload_data(
        self: &Arc<Self>,
        gltf_id: GltfId,
        loader: GltfLoader,
    ) -> Result<GltfData> {
        Ok(loader.into_data()?)
    }

    pub async fn populate(self: &Arc<Self>, data: GltfData) -> Result<()> {
        let mut renderer = self.renderer.lock().await;
        renderer
            .populate_gltf(data, None, self.ctx.generate_mipmaps.get())
            .await?;

        renderer.lights.insert(Light::Directional {
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            direction: [-0.5, -0.25, -0.75],
        });

        Ok(())
    }

    pub async fn render(self: &Arc<Self>) -> Result<()> {
        let mut renderer = self.renderer.lock().await;

        renderer.render()?;

        Ok(())
    }

    pub async fn setup_all(self: &Arc<Self>) -> Result<()> {
        self.last_shader_kind.set(None);
        self.old_shader_kind_material.lock().unwrap().clear();

        self.setup_viewport().await?;

        self.setup_shader().await?;

        Ok(())
    }

    pub async fn setup_viewport(self: &Arc<Self>) -> Result<()> {
        let mut renderer = self.renderer.lock().await;

        let (canvas_width, canvas_height) = renderer.gpu.canvas_size();

        // call these first so we can get the extents
        renderer.update_animations(0.0)?;
        renderer.update_transforms();

        let mut scene_aabb: Option<Aabb> = None;

        for (_, mesh) in renderer.meshes.iter() {
            if let Some(mut mesh_aabb) = mesh.aabb.clone() {
                if let Ok(world_transform) = renderer.transforms.get_world(mesh.transform_key) {
                    mesh_aabb.transform(&*world_transform);
                }
                if let Some(current_scene_aabb) = &mut scene_aabb {
                    current_scene_aabb.extend(&mesh_aabb);
                } else {
                    scene_aabb = Some(mesh_aabb);
                }
            }
        }

        let mut camera = self.camera.lock().unwrap();
        let camera_aspect = canvas_width as f32 / canvas_height as f32;
        if let Some(scene_aabb) = scene_aabb.clone() {
            match self.ctx.camera_id.get() {
                CameraId::Orthographic => {
                    *camera = Some(Camera::new_orthographic(scene_aabb, camera_aspect));
                }
                CameraId::Perspective => {
                    *camera = Some(Camera::new_perspective(scene_aabb, camera_aspect));
                }
            }
        }

        Ok(())
    }

    pub async fn setup_shader(self: &Arc<Self>) -> Result<()> {
        let mut renderer = self.renderer.lock().await;

        let shader_kind = self.ctx.shader.get();

        for mesh_key in renderer.meshes.keys().collect::<Vec<_>>() {
            let render_pipeline_key = renderer.meshes.get(mesh_key)?.render_pipeline_key;

            let mut pipeline_cache_key = renderer
                .pipelines
                .get_render_pipeline_cache_from_key(&render_pipeline_key)
                .ok_or(anyhow::anyhow!(
                    "Failed to get render pipeline cache key from key: {:?}",
                    render_pipeline_key
                ))?;

            let mut shader_cache_key = renderer
                .shaders
                .get_shader_cache_from_key(&pipeline_cache_key.shader_key)
                .ok_or(anyhow::anyhow!(
                    "Failed to get shader cache key from key: {:?}",
                    pipeline_cache_key.shader_key
                ))?;

            let old_shader_kind = match &shader_cache_key.fragment {
                ShaderCacheKeyFragment::Pbr(shader_cache_key_fragment_pbr) => {
                    FragmentShaderKind::Pbr
                }
                ShaderCacheKeyFragment::DebugNormals(shader_cache_key_fragment_debug_normals) => {
                    FragmentShaderKind::DebugNormals
                }
                _ => {
                    return Err(anyhow!(
                        "Unsupported shader kind for meshes: {:?}",
                        shader_cache_key.fragment
                    ));
                }
            };

            if old_shader_kind == shader_kind {
                continue;
            }

            {
                let mut old_shader_kind_material = self.old_shader_kind_material.lock().unwrap();

                let key = (mesh_key.clone(), old_shader_kind);

                if !old_shader_kind_material.contains_key(&key) {
                    old_shader_kind_material.insert(key, shader_cache_key.fragment.clone());
                }
            }

            match shader_kind {
                FragmentShaderKind::DebugNormals => {
                    shader_cache_key.fragment = ShaderCacheKeyFragment::DebugNormals(
                        ShaderCacheKeyFragmentDebugNormals::new(match &shader_cache_key.vertex {
                            awsm_renderer::shaders::vertex::ShaderCacheKeyVertex::Mesh(mesh) => {
                                ShaderTemplateVertexMesh::new(&mesh).has_normals
                            }
                            awsm_renderer::shaders::vertex::ShaderCacheKeyVertex::Quad => false,
                        }),
                    );
                }
                FragmentShaderKind::Pbr => {
                    let fragment = self
                        .old_shader_kind_material
                        .lock()
                        .unwrap()
                        .get(&(mesh_key.clone(), shader_kind))
                        .ok_or(anyhow::anyhow!(
                            "Failed to get material for shader kind {:?}",
                            shader_kind
                        ))?
                        .clone();

                    shader_cache_key.fragment = fragment;
                }
            }

            let shader_key = renderer.add_shader(shader_cache_key).await?;
            pipeline_cache_key.shader_key = shader_key;

            let new_render_pipeline_key = renderer
                .add_render_pipeline(None, pipeline_cache_key)
                .await?;

            renderer.meshes.get_mut(mesh_key)?.render_pipeline_key = new_render_pipeline_key;
        }

        Ok(())
    }

    pub async fn update_all(self: &Arc<Self>, global_time_delta: f64) -> Result<()> {
        let state = self;

        if let Some(camera) = self.camera.lock().unwrap().clone() {
            self.renderer
                .lock()
                .await
                .update_all(global_time_delta, &camera)?;
        }

        Ok(())
    }

    pub fn start_animation_loop(self: &Arc<Self>) {
        let state = self;

        state.stop_animation_loop();
        *state.request_animation_frame.lock().unwrap() = Some(
            gloo_render::request_animation_frame(clone!(state => move |timestamp| {
                state.fire_raf(timestamp);
            })),
        );
    }

    pub fn stop_animation_loop(self: &Arc<Self>) {
        self.request_animation_frame.lock().unwrap().take();
        self.last_request_animation_frame.set(None);
    }

    fn fire_raf(self: &Arc<Self>, timestamp: f64) {
        let state = self;
        spawn_local(clone!(state => async move {
            if let Some(last_timestamp) = state.last_request_animation_frame.get() {
                let time_delta = timestamp - last_timestamp;
                if let Err(err) = state.update_all(time_delta).await {
                    tracing::error!("Failed to animate: {:?}", err);
                }

                if let Err(err) = state.render().await {
                    tracing::error!("Failed to render after animation: {:?}", err);
                }
            }

            let mut lock = state.request_animation_frame.lock().unwrap();

            if lock.take().is_some() {
                state.last_request_animation_frame.set(Some(timestamp));

                *lock = Some(gloo_render::request_animation_frame(clone!(state => move |timestamp| {
                    state.fire_raf(timestamp);
                })));
            }
        }));
    }
}
