use camera::{CameraBuffer, CameraExt};
use mesh::Meshes;
use transform::Transforms;

pub mod camera;
pub mod error;
pub mod mesh;
pub mod render;
pub mod shaders;
pub mod transform;
pub mod core {
    pub use awsm_renderer_core::*;
}
#[cfg(feature = "gltf")]
pub mod gltf;

#[cfg(feature = "animation")]
pub mod animation;

pub struct AwsmRenderer {
    pub gpu: core::renderer::AwsmRendererWebGpu,

    pub meshes: Meshes,

    pub camera: CameraBuffer,

    pub transforms: Transforms,

    #[cfg(feature = "gltf")]
    pub gltf: gltf::cache::GltfCache,

    #[cfg(feature = "animation")]
    pub animations: animation::Animations,
}

impl AwsmRenderer {
    // just a convenience function to update non-GPU properties
    // pair this with .render() once a frame and everything should run smoothly
    // but real-world you may want to update transforms more often for physics, for example
    pub fn update_all(&mut self, time: f64, camera: &impl CameraExt) -> crate::error::Result<()> {
        self.animations.update(time)?;
        self.transforms.update_world()?;
        self.camera.update(camera)?;

        Ok(())
    }
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

    pub fn build(self) -> std::result::Result<AwsmRenderer, crate::error::AwsmError> {
        let gpu = self.gpu.build()?;
        let camera = camera::CameraBuffer::new(&gpu)?;
        let meshes = Meshes::new();
        let transforms = Transforms::new(&gpu)?;

        Ok(AwsmRenderer {
            gpu,
            meshes,
            camera,
            transforms,

            #[cfg(feature = "gltf")]
            gltf: gltf::cache::GltfCache::default(),

            #[cfg(feature = "animation")]
            animations: animation::Animations::default(),
        })
    }
}
