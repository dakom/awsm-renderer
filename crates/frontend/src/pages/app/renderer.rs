use std::collections::HashMap;
use std::ops::Deref;

use awsm_renderer::gltf::data::GltfData;
use awsm_renderer::gltf::loader::GltfLoader;
use awsm_renderer::{AwsmRenderer, AwsmRendererBuilder};
use serde::de;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::models::collections::GltfId;
use crate::pages::app::sidebar::current_model_signal;
use crate::prelude::*;

pub struct AppRenderer {
    pub renderer: futures::lock::Mutex<AwsmRenderer>,
    pub gltf_data: Mutex<HashMap<GltfId, Arc<GltfData>>>,
}

impl AppRenderer {
    pub fn new(renderer: AwsmRenderer) -> Arc<Self> {
        Arc::new(Self {
            renderer: futures::lock::Mutex::new(renderer),
            gltf_data: Mutex::new(HashMap::new()),
        })
    }

    pub async fn render(self: &Arc<Self>, gltf_id: GltfId) -> Result<()> {
        tracing::info!("Rendering GLTF model: {:?}", gltf_id);

        let state = self;

        let url = format!("{}/{}", CONFIG.gltf_url, gltf_id.filepath());

        let gltf_data = { state.gltf_data.lock().unwrap().get(&gltf_id).cloned() };

        let gltf_data = match gltf_data {
            Some(gltf_data) => gltf_data,
            None => {
                let gltf_loader = GltfLoader::load(&url, None).await?;
                let lock = state.renderer.lock().await;
                let gltf_data = Arc::new(GltfData::new(&lock, gltf_loader).await?);
                {
                    state
                        .gltf_data
                        .lock()
                        .unwrap()
                        .insert(gltf_id, gltf_data.clone());
                }

                gltf_data
            }
        };

        {
            let mut lock = state.renderer.lock().await;
            lock.populate_gltf(gltf_data, None).await?;
            lock.render()?;
        }

        Ok(())
    }
}
