use buffers::storage::StorageBuffers;
use camera::CameraBuffer;
use mesh::Meshes;
use skin::Skins;
use transform::Transforms;

pub mod bounds;
pub mod buffers;
pub mod camera;
pub mod error;
pub mod mesh;
pub mod render;
pub mod shaders;
pub mod transform;
pub mod skin;
pub mod update;
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

    pub skins: Skins,

    pub storage: StorageBuffers,

    #[cfg(feature = "gltf")]
    gltf: gltf::cache::GltfCache,

    #[cfg(feature = "animation")]
    pub animations: animation::Animations,
}

impl AwsmRenderer {
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
        let skins = Skins::new(&gpu)?;

        Ok(AwsmRenderer {
            gpu,
            meshes,
            camera,
            transforms,
            skins,
            storage: StorageBuffers::new(),

            #[cfg(feature = "gltf")]
            gltf: gltf::cache::GltfCache::default(),

            #[cfg(feature = "animation")]
            animations: animation::Animations::default(),
        })
    }
}
