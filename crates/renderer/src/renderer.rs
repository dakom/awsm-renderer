use awsm_renderer_core::renderer::{AwsmRendererWebGpu, AwsmRendererWebGpuBuilder};
use awsm_renderer_scene::renderer::AwsmRendererScene;
use anyhow::Result;

pub struct AwsmRenderer {
    pub gpu: AwsmRendererWebGpu,
    pub scene: AwsmRendererScene,
}


pub struct AwsmRendererBuilder {
    core: AwsmRendererWebGpuBuilder,
}

impl AwsmRendererBuilder {
    pub fn new(gpu: web_sys::Gpu) -> Self {
        Self {
            core: AwsmRendererWebGpuBuilder::new(gpu),
        }
    }

    pub async fn init_adapter(mut self) -> Result<Self> {
        self.core = self.core.init_adapter().await?;
        Ok(self)
    }

    pub async fn init_device(mut self) -> Result<Self> {
        self.core = self.core.init_device().await?;
        Ok(self)
    }

    pub fn init_context(mut self, canvas: web_sys::HtmlCanvasElement) -> Result<Self> {
        self.core = self.core.init_context(canvas)?;
        Ok(self)
    }

    pub fn build(self) -> Result<AwsmRenderer> {
        let core = self.core.build()?;
        let scene = AwsmRendererScene::new(core.clone());
        Ok(AwsmRenderer { gpu: core, scene })
    }
}