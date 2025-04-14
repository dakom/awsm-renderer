mod camera;

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Deref;

use awsm_renderer::gltf::data::GltfData;
use awsm_renderer::gltf::loader::GltfLoader;
use awsm_renderer::mesh::PositionExtents;
use awsm_renderer::{AwsmRenderer, AwsmRendererBuilder};
use awsm_web::dom::resize::ResizeObserver;
use camera::Camera;
use glam::Vec3;
use serde::de;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::models::collections::GltfId;
use crate::pages::app::sidebar::current_model_signal;
use crate::prelude::*;

pub struct AppScene {
    pub renderer: futures::lock::Mutex<AwsmRenderer>,
    pub gltf_loader: Mutex<HashMap<GltfId, GltfLoader>>,
    pub camera: Mutex<Camera>,
    pub resize_observer: Mutex<Option<ResizeObserver>>,
    pub request_animation_frame: Mutex<Option<gloo_render::AnimationFrame>>,
    pub last_request_animation_frame: Cell<Option<f64>>,
}

impl AppScene {
    pub fn new(renderer: AwsmRenderer, canvas: web_sys::HtmlCanvasElement) -> Arc<Self> {
        let state = Arc::new(Self {
            renderer: futures::lock::Mutex::new(renderer),
            gltf_loader: Mutex::new(HashMap::new()),
            camera: Mutex::new(Camera::default()),
            resize_observer: Mutex::new(None),
            request_animation_frame: Mutex::new(None),
            last_request_animation_frame: Cell::new(None),
        });

        let resize_observer = ResizeObserver::new(
            clone!(canvas, state => move |entries| {
                if let Some(entry) = entries.get(0) {
                    let width = entry.content_box_sizes[0].inline_size;
                    let height = entry.content_box_sizes[0].block_size;
                    canvas.set_width(width);
                    canvas.set_height(height);

                    spawn_local(clone!(state => async move {
                        if let Err(err) = state.setup().await {
                            tracing::error!("Failed to setup scene after canvas resize: {:?}", err);
                        }

                        if let Err(err) = state.render().await {
                            tracing::error!("Failed to render after canvas resize: {:?}", err);
                        }
                    }));
                }
            }),
            None,
        );

        resize_observer.observe(&canvas);

        *state.resize_observer.lock().unwrap() = Some(resize_observer);

        state
    }

    pub async fn clear(self: &Arc<Self>) {
        let state = self;

        let mut lock = state.renderer.lock().await;

        lock.meshes.clear();
        lock.gltf.raw_datas.clear();
        state.stop_animation_loop();
        lock.render();
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
        let state = self;

        let lock = state.renderer.lock().await;
        Ok(GltfData::new(&lock, loader).await?)
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
        renderer.update_transforms()?;

        let mut extents: Option<PositionExtents> = None;

        for mesh in renderer.meshes.iter() {
            if let Some(mut mesh_extents) = mesh.position_extents.clone() {
                if let Ok(world_transform) = renderer.transforms.get_world(mesh.transform_key) {
                    mesh_extents.apply_matrix(&*world_transform);
                }
                if let Some(mut current_extents) = extents {
                    current_extents.extend(&mesh_extents);
                    extents = Some(current_extents);
                } else {
                    extents = Some(mesh_extents);
                }
            }
        }

        let mut camera = self.camera.lock().unwrap();
        if let Some(extents) = extents {
            camera.set_extents(extents);
        }

        Ok(())
    }

    pub async fn update_all(self: &Arc<Self>, global_time_delta: f64) -> Result<()> {
        let state = self;

        let camera = self.camera.lock().unwrap();

        self.renderer
            .lock()
            .await
            .update_all(global_time_delta, &*camera)?;

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
