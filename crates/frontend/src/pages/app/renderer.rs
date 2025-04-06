use std::collections::HashMap;
use std::ops::Deref;

use awsm_renderer::gltf::data::GltfData;
use awsm_renderer::gltf::loader::GltfLoader;
use awsm_renderer::{AwsmRenderer, AwsmRendererBuilder};
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::models::collections::GltfId;
use crate::prelude::*;

#[derive(Clone)]
pub struct AppRenderer {
    inner: Arc<AppRendererInner>,
}

struct AppRendererInner {
    pub renderer: Mutex<Option<AwsmRenderer>>,
    pub gltf_id: Mutex<Option<GltfId>>,
    pub pipelines: Mutex<HashMap<GltfId, web_sys::GpuRenderPipeline>>,
    pub gltf_data: Mutex<HashMap<GltfId, Arc<GltfData>>>,
}

impl AppRenderer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AppRendererInner {
                renderer: Mutex::new(None),
                gltf_id: Mutex::new(None),
                pipelines: Mutex::new(HashMap::new()),
                gltf_data: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn set_model(&self, model_id: GltfId) {
        let inner = self.inner.clone();
        spawn_local(async move {
            {
                *inner.gltf_id.lock().unwrap() = Some(model_id);
            }
            inner.render().await;
        });
    }

    pub fn set_canvas(&self, canvas: web_sys::HtmlCanvasElement) {
        let inner = self.inner.clone();
        spawn_local(async move {
            let renderer = AwsmRendererBuilder::new(web_sys::window().unwrap().navigator().gpu())
                .init_adapter()
                .await
                .unwrap()
                .init_device()
                .await
                .unwrap()
                .init_context(canvas.clone())
                .unwrap()
                .build()
                .unwrap();

            {
                *inner.renderer.lock().unwrap() = Some(renderer);
            }

            inner.render().await;
        });
    }
}

impl AppRendererInner {
    async fn render(&self) {
        if let Err(err) = self.inner_render().await {
            tracing::error!("{:?}", err);
        }
    }
    async fn inner_render(&self) -> Result<()> {
        match (
            &mut *self.renderer.lock().unwrap(),
            *self.gltf_id.lock().unwrap(),
        ) {
            (Some(renderer), Some(gltf_id)) => {
                let url = format!("{}/{}", CONFIG.gltf_url, gltf_id.filepath());

                let gltf_data = { self.gltf_data.lock().unwrap().get(&gltf_id).cloned() };

                let gltf_data = match gltf_data {
                    Some(gltf_data) => gltf_data,
                    None => {
                        let gltf_loader = GltfLoader::load(&url, None).await?;
                        let gltf_data = Arc::new(GltfData::new(renderer, gltf_loader).await?);

                        self.gltf_data
                            .lock()
                            .unwrap()
                            .insert(gltf_id, gltf_data.clone());

                        gltf_data
                    }
                };

                renderer.populate_gltf(gltf_data, None).await?;

                renderer.render()?;

                loop {
                    JsFuture::from(renderer.gpu.device.lost()).await;
                    tracing::info!("GPU device lost, attempting re-render");
                    renderer.render()?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
