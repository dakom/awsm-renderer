pub mod camera;

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Deref;

use awsm_renderer::bounds::Aabb;
use awsm_renderer::gltf::data::GltfData;
use awsm_renderer::gltf::loader::GltfLoader;
use awsm_renderer::{AwsmRenderer, AwsmRendererBuilder};
use awsm_web::dom::resize::ResizeObserver;
use camera::{Camera, CameraId};
use glam::Vec3;
use gloo_events::EventListener;
use serde::de;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::models::collections::GltfId;
use crate::pages::app::sidebar::current_model_signal;
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
                        let event = event.unchecked_ref::<web_sys::PointerEvent>();
                        camera.on_pointer_down(event.client_x(), event.client_y());
                    }
                }),
            ),
            EventListener::new(
                &web_sys::window().unwrap(),
                "pointermove",
                clone!(state => move |event| {
                    if let Some(camera) = state.camera.lock().unwrap().as_mut() {
                        let event = event.unchecked_ref::<web_sys::PointerEvent>();
                        camera.on_pointer_move(event.client_x(), event.client_y());
                    }
                }),
            ),
            EventListener::new(
                &web_sys::window().unwrap(),
                "pointerup",
                clone!(state => move |event| {
                    if let Some(camera) = state.camera.lock().unwrap().as_mut() {
                        let event = event.unchecked_ref::<web_sys::PointerEvent>();
                        camera.on_pointer_up(event.client_x(), event.client_y());
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

        state
    }

    fn on_viewport_change(self: &Arc<Self>) {
        let state = self;

        spawn_local(clone!(state => async move {
            let aspect = {
                let renderer = state.renderer.lock().await;
                let canvas = renderer.gpu.canvas();
                canvas.width() as f32 / canvas.height() as f32
            };

            if let Err(err) = state.setup().await {
                tracing::error!("Failed to setup scene after canvas resize: {:?}", err);
            }

            if let Err(err) = state.render().await {
                tracing::error!("Failed to render after canvas resize: {:?}", err);
            }
        }));
    }

    pub async fn clear(self: &Arc<Self>) {
        let state = self;

        let mut renderer = state.renderer.lock().await;

        state.stop_animation_loop();
        if let Err(err) = renderer.remove_all() {
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
        self.renderer.lock().await.populate_gltf(data, None).await
    }

    pub async fn render(self: &Arc<Self>) -> Result<()> {
        let mut renderer = self.renderer.lock().await;

        renderer.render()?;

        Ok(())
    }

    pub async fn setup(self: &Arc<Self>) -> Result<()> {
        let mut renderer = self.renderer.lock().await;

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

        let aspect = renderer.gpu.canvas().width() as f32 / renderer.gpu.canvas().height() as f32;

        let mut camera = self.camera.lock().unwrap();
        if let Some(scene_aabb) = scene_aabb.clone() {
            match self.ctx.camera_id.get() {
                CameraId::Orthographic => {
                    *camera = Some(Camera::new_orthographic(scene_aabb, aspect));
                }
                CameraId::Perspective => {
                    *camera = Some(Camera::new_perspective(scene_aabb, aspect));
                }
            }
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
