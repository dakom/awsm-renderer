use buffers::storage::StorageBuffers;
use camera::{CameraBuffer, CameraExt};
use mesh::Meshes;
use transform::Transforms;

pub mod camera;
pub mod error;
pub mod mesh;
pub mod render;
pub mod buffers;
pub mod transform;
pub mod shaders;
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

    pub storage: StorageBuffers,

    #[cfg(feature = "gltf")]
    pub gltf: gltf::cache::GltfCache,

    #[cfg(feature = "animation")]
    pub animations: animation::Animations,
}

impl AwsmRenderer {
    // just a convenience function to update non-GPU properties
    // pair this with .render() once a frame and everything should run smoothly
    // but real-world you may want to update transforms more often for physics, for example
    pub fn update_all(
        &mut self,
        global_time_delta: f64,
        camera: &impl CameraExt,
    ) -> crate::error::Result<()> {
        self.update_animations(global_time_delta)?;
        self.update_transforms()?;
        self.update_camera(camera)?;

        Ok(())
    }

    pub fn remove_all(&mut self) -> crate::error::Result<()> {
        self.camera = camera::CameraBuffer::new(&self.gpu)?;
        self.meshes = Meshes::new(&self.gpu)?;
        self.transforms = Transforms::new(&self.gpu)?;

        #[cfg(feature = "gltf")]
        {
            self.gltf = gltf::cache::GltfCache::default();
        }

        #[cfg(feature = "animation")]
        {
            self.animations = animation::Animations::default();
        }

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
        let meshes = Meshes::new(&gpu)?;
        let transforms = Transforms::new(&gpu)?;

        Ok(AwsmRenderer {
            gpu,
            meshes,
            camera,
            transforms,
            storage: StorageBuffers::new(),

            #[cfg(feature = "gltf")]
            gltf: gltf::cache::GltfCache::default(),

            #[cfg(feature = "animation")]
            animations: animation::Animations::default(),
        })
    }
}
