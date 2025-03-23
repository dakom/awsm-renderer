
#[cfg(feature = "gltf")]
pub mod gltf;
pub mod wip;
pub mod transform;
pub mod render;
pub mod camera;
pub mod core {
    pub use awsm_renderer_core::*;
}

pub struct AwsmRenderer {
    pub gpu: core::renderer::AwsmRendererWebGpu,
}


pub struct AwsmRendererBuilder {
    gpu : core::renderer::AwsmRendererWebGpuBuilder,
}

impl AwsmRendererBuilder {
    pub fn new(gpu: web_sys::Gpu) -> Self {
        Self {
            gpu : core::renderer::AwsmRendererWebGpuBuilder::new(gpu),
        }
    }

    pub async fn init_adapter(mut self) -> core::error::Result<Self> {
        self.gpu = self.gpu.init_adapter().await?;
        Ok(self)
    }

    pub async fn init_device(mut self) -> core::error::Result<Self> {
        self.gpu = self.gpu.init_device().await?;
        Ok(self)
    }

    pub fn init_context(mut self, canvas: web_sys::HtmlCanvasElement) -> core::error::Result<Self> {
        self.gpu = self.gpu.init_context(canvas)?;
        Ok(self)
    }

    pub fn build(self) -> core::error::Result<AwsmRenderer> {
        let gpu = self.gpu.build()?;
        Ok(AwsmRenderer { gpu })
    }
}