mod camera;

use std::collections::HashMap;
use std::ops::Deref;

use awsm_renderer::gltf::data::GltfData;
use awsm_renderer::gltf::loader::GltfLoader;
use awsm_renderer::mesh::PositionExtents;
use awsm_renderer::{AwsmRenderer, AwsmRendererBuilder};
use camera::Camera;
use serde::de;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::models::collections::GltfId;
use crate::pages::app::sidebar::current_model_signal;
use crate::prelude::*;

pub struct AppScene {
    pub renderer: futures::lock::Mutex<AwsmRenderer>,
    pub gltf_loader: Mutex<HashMap<GltfId, GltfLoader>>,
    pub camera: Mutex<Camera>,
}

impl AppScene {
    pub fn new(renderer: AwsmRenderer) -> Arc<Self> {
        Arc::new(Self {
            renderer: futures::lock::Mutex::new(renderer),
            gltf_loader: Mutex::new(HashMap::new()),
            camera: Mutex::new(Camera::default()),
        })
    }

    pub async fn clear(self: &Arc<Self>) {
        let state = self;

        let mut lock = state.renderer.lock().await;

        lock.meshes.clear();
        lock.gltf.raw_datas.clear();
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
        Ok(self.renderer.lock().await.render()?)
    }

    pub async fn reset_camera(self: &Arc<Self>) -> Result<()> {
        let lock = self.renderer.lock().await;
        let mut extents: Option<PositionExtents> = None;

        for mesh in lock.meshes.iter() {
            if let Some(mesh_extents) = &mesh.position_extents {
                if let Some(mut current_extents) = extents {
                    current_extents.extend(mesh_extents);
                    extents = Some(current_extents);
                } else {
                    extents = Some(mesh_extents.clone());
                }
            }
        }

        let mut camera = self.camera.lock().unwrap();
        if let Some(extents) = extents {
            camera.set_extents(extents);
        }

        lock.camera_buffer.write(&*camera)?;

        Ok(())
    }
}
