pub mod camera;
pub mod error;
#[cfg(feature = "gltf")]
pub mod gltf;
pub mod mesh;
pub mod render;
pub mod transform;
pub mod core {
    pub use awsm_renderer_core::*;
}

pub struct AwsmRenderer {
    pub gpu: core::renderer::AwsmRendererWebGpu,

    #[cfg(feature = "gltf")]
    pub gltf: gltf::cache::GltfCache,

    pub meshes: mesh::Meshes,

    pub camera_buffer: camera::CameraBuffer,
}

pub struct AwsmRendererBuilder {
    gpu: core::renderer::AwsmRendererWebGpuBuilder,
}

impl AwsmRendererBuilder {
    pub fn new(gpu: web_sys::Gpu) -> Self {
        Self {
            gpu: core::renderer::AwsmRendererWebGpuBuilder::new(gpu),
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

    #[cfg(feature = "gltf")]
    pub fn build(self) -> std::result::Result<AwsmRenderer, crate::error::AwsmError> {
        let gpu = self.gpu.build()?;
        let camera_buffer = camera::CameraBuffer::new(gpu.clone())?;

        Ok(AwsmRenderer {
            gpu,
            gltf: gltf::cache::GltfCache::default(),
            meshes: mesh::Meshes::default(),
            camera_buffer
        })
    }

    #[cfg(not(feature = "gltf"))]
    pub fn build(self) -> std::result::Result<AwsmRenderer, crate::error::AwsmError> {
        let gpu = self.gpu.build()?;
        let camera_buffer = camera::CameraBuffer::new(gpu.clone())?;

        Ok(AwsmRenderer {
            gpu,
            meshes: mesh::Meshes::default(),
            camera_buffer
        })
    }
}
