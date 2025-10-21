pub mod camera;
mod ibl;
mod skybox;

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Deref;

use awsm_renderer::bounds::Aabb;
use awsm_renderer::core::command::color::Color;
use awsm_renderer::core::cubemap::images::CubemapBitmapColors;
use awsm_renderer::core::cubemap::CubemapImage;
use awsm_renderer::core::renderer;
use awsm_renderer::core::texture::TextureFormat;
use awsm_renderer::environment::Skybox;
use awsm_renderer::gltf::data::GltfData;
use awsm_renderer::gltf::loader::GltfLoader;
use awsm_renderer::lights::ibl::IblTexture;
use awsm_renderer::lights::{ibl::Ibl, Light};
use awsm_renderer::mesh::MeshKey;
use awsm_renderer::{AwsmRenderer, AwsmRendererBuilder};
use awsm_web::dom::resize::ResizeObserver;
use camera::{Camera, CameraId};
use glam::Vec3;
use gloo_events::EventListener;
use serde::de;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::models::collections::GltfId;
use crate::pages::app::context::{IblId, SkyboxId};
use crate::pages::app::sidebar::current_model_signal;
use crate::pages::app::sidebar::material::FragmentShaderKind;
use crate::prelude::*;

use super::canvas;
use super::context::AppContext;

pub struct AppScene {
    pub ctx: AppContext,
    pub renderer: futures::lock::Mutex<AwsmRenderer>,
    pub gltf_cache: Mutex<HashMap<GltfId, GltfLoader>>,
    pub latest_gltf_data: Mutex<Option<GltfData>>,
    pub ibl_cache: Mutex<HashMap<IblId, Ibl>>,
    pub skybox_by_ibl_cache: Mutex<HashMap<IblId, Skybox>>,
    pub camera: Mutex<Option<Camera>>,
    pub resize_observer: Mutex<Option<ResizeObserver>>,
    pub request_animation_frame: Mutex<Option<gloo_render::AnimationFrame>>,
    pub last_request_animation_frame: Cell<Option<f64>>,
    pub event_listeners: Mutex<Vec<EventListener>>,
    last_size: Cell<(f64, f64)>,
    last_camera_id: Cell<CameraId>,
    last_shader_kind: Cell<Option<FragmentShaderKind>>,
}

impl AppScene {
    pub fn new(ctx: AppContext, renderer: AwsmRenderer) -> Arc<Self> {
        let canvas = renderer.gpu.canvas();

        let state = Arc::new(Self {
            ctx,
            renderer: futures::lock::Mutex::new(renderer),
            gltf_cache: Mutex::new(HashMap::new()),
            ibl_cache: Mutex::new(HashMap::new()),
            skybox_by_ibl_cache: Mutex::new(HashMap::new()),
            latest_gltf_data: Mutex::new(None),
            camera: Mutex::new(None),
            resize_observer: Mutex::new(None),
            request_animation_frame: Mutex::new(None),
            last_request_animation_frame: Cell::new(None),
            event_listeners: Mutex::new(Vec::new()),
            last_size: Cell::new((0.0, 0.0)),
            last_camera_id: Cell::new(CameraId::default()),
            last_shader_kind: Cell::new(None),
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
            state.ctx.ibl_id.signal().for_each(clone!(state => move |ibl_id| clone!(state => async move {
                if let Err(e) = state.load_ibl(ibl_id).await {
                    tracing::error!("Failed to load IBL {:?}: {:?}", ibl_id, e);
                }

                match state.ctx.skybox_id.get() {
                    SkyboxId::SameAsIbl => {
                        if let Err(e) = state.load_skybox(SkyboxId::SameAsIbl).await {
                            tracing::error!("Failed to load Skybox {:?}: {:?}", ibl_id, e);
                        }
                    }
                }
            }))).await;
        }));

        spawn_local(clone!(state => async move {
            state.ctx.skybox_id.signal().for_each(clone!(state => move |skybox_id| clone!(state => async move {
                if let Err(e) = state.load_skybox(skybox_id).await {
                    tracing::error!("Failed to load Skybox {:?}: {:?}", skybox_id, e);
                }
            }))).await;
        }));

        state
    }

    fn on_viewport_change(self: &Arc<Self>) {
        let state = self;

        spawn_local(clone!(state => async move {
            let last_size = state.last_size.get();
            let last_camera_id = state.last_camera_id.get();
            let camera_id = state.ctx.camera_id.get();

            {
                let renderer = state.renderer.lock().await;
                let (canvas_width, canvas_height) = renderer.gpu.canvas_size();
                if (canvas_width, canvas_height) == last_size && camera_id == last_camera_id {
                    return;
                }
                state.last_size.set((canvas_width, canvas_height));
                state.last_camera_id.set(camera_id);
            }


            if let Err(err) = state.setup_viewport().await {
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
        if let Err(err) = renderer.remove_all().await {
            tracing::error!("Failed to clear renderer: {:?}", err);
        }
        renderer.render();
    }

    pub async fn load_gltf(self: &Arc<Self>, gltf_id: GltfId) -> Result<GltfLoader> {
        let state = self;

        if let Some(loader) = state
            .gltf_cache
            .lock()
            .unwrap()
            .get(&gltf_id)
            .map(|loader| loader.heavy_clone())
        {
            return Ok(loader);
        }

        let url = format!("{}/{}", CONFIG.gltf_url, gltf_id.filepath());

        let loader = GltfLoader::load(&url, None).await?;

        state
            .gltf_cache
            .lock()
            .unwrap()
            .insert(gltf_id, loader.heavy_clone());

        Ok(loader)
    }

    async fn load_skybox(self: &Arc<Self>, skybox_id: SkyboxId) -> Result<()> {
        let skybox = match skybox_id {
            SkyboxId::SameAsIbl => {
                let ibl_id = self.ctx.ibl_id.get_cloned();
                let maybe_cached = {
                    // need to drop this lock before awaiting
                    self.skybox_by_ibl_cache
                        .lock()
                        .unwrap()
                        .get(&ibl_id)
                        .cloned()
                };
                match maybe_cached {
                    Some(skybox) => skybox,
                    None => {
                        let skybox_cubemap = match ibl_id {
                            IblId::PhotoStudio => skybox::load_from_path("photo_studio").await?,
                            IblId::AllWhite => {
                                skybox::load_from_colors(CubemapBitmapColors::all(Color::WHITE))
                                    .await?
                            }
                            IblId::SimpleSky => skybox::load_simple_sky().await?,
                        };

                        let skybox = {
                            let (texture, view, mip_count) = {
                                let mut renderer = &mut *self.renderer.lock().await;
                                skybox_cubemap
                                    .create_texture_and_view(&renderer.gpu, Some("Skybox"))
                                    .await?
                            };

                            {
                                let mut renderer = &mut *self.renderer.lock().await;
                                let key = renderer.textures.insert_cubemap(texture);

                                let sampler_key = renderer
                                    .textures
                                    .get_sampler_key(&renderer.gpu, Skybox::sampler_cache_key())?;

                                let sampler = renderer.textures.get_sampler(sampler_key)?.clone();

                                Skybox::new(key, view, sampler, mip_count)
                            }
                        };

                        self.skybox_by_ibl_cache
                            .lock()
                            .unwrap()
                            .insert(ibl_id, skybox.clone());

                        skybox
                    }
                }
            }
        };

        self.renderer.lock().await.set_skybox(skybox);

        Ok(())
    }

    pub async fn wait_for_skybox_loaded(self: &Arc<Self>) {
        loop {
            let skybox_id = self.ctx.skybox_id.get_cloned();
            let skybox_loaded = {
                match skybox_id {
                    SkyboxId::SameAsIbl => {
                        let ibl_id = self.ctx.ibl_id.get_cloned();
                        let skybox_cache = self.skybox_by_ibl_cache.lock().unwrap();
                        skybox_cache.contains_key(&ibl_id)
                    }
                }
            };

            if skybox_loaded {
                break;
            }

            gloo_timers::future::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    pub async fn load_ibl(self: &Arc<Self>, ibl_id: IblId) -> Result<()> {
        async fn create_ibl_texture(
            renderer: &mut AwsmRenderer,
            cubemap_image: CubemapImage,
        ) -> Result<IblTexture> {
            let (texture, view, mip_count) = cubemap_image
                .create_texture_and_view(&renderer.gpu, Some("IBL Cubemap"))
                .await?;

            let texture_key = renderer.textures.insert_cubemap(texture);

            let sampler_key = renderer
                .textures
                .get_sampler_key(&renderer.gpu, IblTexture::sampler_cache_key())?;

            let sampler = renderer.textures.get_sampler(sampler_key)?.clone();

            Ok(IblTexture::new(texture_key, view, sampler, mip_count))
        }

        let ibl = {
            let maybe_cached = {
                // need to drop this lock before awaiting
                self.ibl_cache.lock().unwrap().get(&ibl_id).cloned()
            };

            match maybe_cached {
                Some(ibl) => ibl.clone(),
                None => {
                    let ibl_cubemaps = match ibl_id {
                        IblId::PhotoStudio => ibl::load_from_path("photo_studio").await?,
                        IblId::AllWhite => {
                            ibl::load_from_colors(CubemapBitmapColors::all(Color::WHITE)).await?
                        }
                        IblId::SimpleSky => ibl::load_simple_sky().await?,
                    };

                    let ibl = {
                        let mut renderer = self.renderer.lock().await;

                        let prefiltered_env_texture =
                            create_ibl_texture(&mut renderer, ibl_cubemaps.prefiltered_env).await?;
                        let irradiance_texture =
                            create_ibl_texture(&mut renderer, ibl_cubemaps.irradiance).await?;

                        Ibl::new(prefiltered_env_texture, irradiance_texture)
                    };

                    self.ibl_cache.lock().unwrap().insert(ibl_id, ibl.clone());

                    ibl
                }
            }
        };

        self.renderer.lock().await.set_ibl(ibl.clone());

        Ok(())
    }

    pub async fn wait_for_ibl_loaded(self: &Arc<Self>) {
        loop {
            let ibl_id = self.ctx.ibl_id.get_cloned();
            let ibl_loaded = {
                let ibl_cache = self.ibl_cache.lock().unwrap();
                ibl_cache.contains_key(&ibl_id)
            };

            if ibl_loaded {
                break;
            }

            gloo_timers::future::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    pub async fn upload_data(self: &Arc<Self>, gltf_id: GltfId, loader: GltfLoader) -> Result<()> {
        let data = loader.into_data()?;

        *self.latest_gltf_data.lock().unwrap() = Some(data);

        Ok(())
    }

    pub async fn populate(self: &Arc<Self>) -> Result<()> {
        let data = self.latest_gltf_data.lock().unwrap();
        let data = data
            .as_ref()
            .expect("No GLTF data to populate")
            .heavy_clone();

        let mut renderer = self.renderer.lock().await;

        renderer
            .populate_gltf(data, None, self.ctx.generate_mipmaps.get())
            .await?;

        // TODO: Disable punctual lights to see pure IBL contribution
        // Once IBL is properly implemented, restore this with proper intensity
        // renderer.lights.insert(Light::Directional {
        //     color: [1.0, 1.0, 1.0],
        //     intensity: 1.0,
        //     direction: [-0.5, -0.25, -0.75],
        // });

        if let Some(ibl) = self
            .ibl_cache
            .lock()
            .unwrap()
            .get(&self.ctx.ibl_id.get())
            .cloned()
        {
            renderer.set_ibl(ibl);
        }

        if let Some(skybox) = self
            .skybox_by_ibl_cache
            .lock()
            .unwrap()
            .get(&self.ctx.ibl_id.get())
            .cloned()
        {
            renderer.set_skybox(skybox);
        }

        Ok(())
    }

    pub async fn render(self: &Arc<Self>) -> Result<()> {
        let mut renderer = self.renderer.lock().await;

        renderer.render()?;

        Ok(())
    }

    pub async fn setup_all(self: &Arc<Self>) -> Result<()> {
        self.last_shader_kind.set(None);

        self.setup_viewport().await?;

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

        let gltf_doc = self
            .latest_gltf_data
            .lock()
            .unwrap()
            .as_ref()
            .map(|data| data.doc.clone());

        let camera_aspect = canvas_width as f32 / canvas_height as f32;
        let mut camera = self.camera.lock().unwrap();
        match self.ctx.camera_id.get() {
            CameraId::Orthographic => {
                *camera = Some(Camera::new_orthographic(
                    scene_aabb,
                    gltf_doc,
                    camera_aspect,
                ));
            }
            CameraId::Perspective => {
                tracing::info!("setting new perspective camera");
                *camera = Some(Camera::new_perspective(scene_aabb, gltf_doc, camera_aspect));
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
                .update_all(global_time_delta, camera.matrices())?;
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
