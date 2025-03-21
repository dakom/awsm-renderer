use std::collections::HashMap;
use std::ops::Deref;

use awsm_renderer::renderer::{AwsmRenderer, AwsmRendererBuilder};
use awsm_renderer::wip::AwsmRendererWipExt;
use wasm_bindgen_futures::spawn_local;

use crate::prelude::*;
use crate::models::collections::GltfId;

#[derive(Clone)]
pub struct AppRenderer {
    inner: Arc<AppRendererInner>,
}

struct AppRendererInner {
    pub renderer: Mutex<Option<AwsmRenderer>>,
    pub gltf_id: Mutex<Option<GltfId>>,
    pub pipelines: Mutex<HashMap<GltfId, web_sys::GpuRenderPipeline>>,
}

impl AppRenderer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AppRendererInner {
                renderer: Mutex::new(None),
                gltf_id: Mutex::new(None),
                pipelines: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn set_model(&self, model_id: GltfId) {
        let inner = self.inner.clone();
        spawn_local(async move {
            *inner.gltf_id.lock().unwrap() = Some(model_id);
            inner.render().await;
        });
    }

    pub fn set_canvas(&self, canvas: web_sys::HtmlCanvasElement) {
        let inner = self.inner.clone();
        spawn_local(async move {
            let mut renderer = AwsmRendererBuilder::new(web_sys::window().unwrap().navigator().gpu())
                .init_adapter().await.unwrap()
                .init_device().await.unwrap()
                .init_context(canvas.clone()).unwrap()
                .build()
                .unwrap();

            *inner.renderer.lock().unwrap() = Some(renderer);

            inner.render().await;
        });
    }

}

impl AppRendererInner {
    async fn render(&self) {
        match (&mut *self.renderer.lock().unwrap(), *self.gltf_id.lock().unwrap()) {
            (Some(renderer), Some(gltf_id)) => {
                tracing::info!("Rendering model with ID: {:?}", gltf_id);
                renderer.temp_render().await.unwrap();

                // let pipeline = {
                //     self.pipelines.lock().unwrap().get(&gltf_id).cloned()
                // };

                // let pipeline = match pipeline {
                //     Some(pipeline) => pipeline,
                //     None => {
                //         let pipeline = renderer.temp_pipeline().await.unwrap();
                //         self.pipelines.lock().unwrap().insert(gltf_id, pipeline.clone());
                //         pipeline
                //     }
                // };

                // let mut commands = CommandBuilder::new(None);

                // renderer.temp_render(commands.build(renderer).unwrap());
            }
            _ => {}
        }
    }
}