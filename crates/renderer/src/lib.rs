use camera::CameraBuffer;
use slotmap::SlotMap;
use transform::Transforms;

pub mod camera;
pub mod error;
#[cfg(feature = "gltf")]
pub mod gltf;
pub mod mesh;
pub mod render;
pub mod shaders;
pub mod transform;
pub mod core {
    pub use awsm_renderer_core::*;
}

pub struct AwsmRenderer {
    pub gpu: core::renderer::AwsmRendererWebGpu,

    #[cfg(feature = "gltf")]
    pub gltf: gltf::cache::GltfCache,

    pub meshes: SlotMap<mesh::MeshKey, mesh::Mesh>,

    pub camera: CameraBuffer,

    pub transforms: Transforms,
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
        let camera = camera::CameraBuffer::new(&gpu)?;

        Ok(AwsmRenderer {
            gpu,
            gltf: gltf::cache::GltfCache::default(),
            meshes: SlotMap::with_key(),
            camera,
            transforms: Transforms::default(),
        })
    }

    #[cfg(not(feature = "gltf"))]
    pub fn build(self) -> std::result::Result<AwsmRenderer, crate::error::AwsmError> {
        let gpu = self.gpu.build()?;
        let camera_buffer = camera::CameraBuffer::new(&gpu)?;

        Ok(AwsmRenderer {
            gpu,
            meshes: SlotMap::with_key(),
            camera_buffer,
            transforms: Transforms::default(),
        })
    }
}
