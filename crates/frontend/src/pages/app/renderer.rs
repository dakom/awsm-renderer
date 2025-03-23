use std::collections::HashMap;
use std::ops::Deref;

use awsm_renderer::gltf::loader::GltfResource;
use awsm_renderer::{AwsmRenderer, AwsmRendererBuilder};
use wasm_bindgen_futures::spawn_local;

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
    pub gltf_res: Mutex<HashMap<GltfId, GltfResource>>,
}

impl AppRenderer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AppRendererInner {
                renderer: Mutex::new(None),
                gltf_id: Mutex::new(None),
                pipelines: Mutex::new(HashMap::new()),
                gltf_res: Mutex::new(HashMap::new()),
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
    async fn render(&self) -> Result<()> {
        match (
            &mut *self.renderer.lock().unwrap(),
            *self.gltf_id.lock().unwrap(),
        ) {
            (Some(renderer), Some(gltf_id)) => {
                let url = format!("{}/{}", CONFIG.gltf_url, gltf_id.filepath());
                tracing::info!("Rendering model at: {}", url);

                let gltf_res = { self.gltf_res.lock().unwrap().get(&gltf_id).cloned() };

                let gltf_res = match gltf_res {
                    Some(gltf_res) => gltf_res,
                    None => {
                        let gltf_res = GltfResource::load(&url, None).await?;
                        self.gltf_res
                            .lock()
                            .unwrap()
                            .insert(gltf_id, gltf_res.clone());
                        gltf_res
                    }
                };

                renderer.populate_gltf(&gltf_res).await?;

                renderer.render()?;
            }
            _ => {}
        }

        Ok(())
    }
}
