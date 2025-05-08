use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use bind_groups::BindGroups;
use camera::CameraBuffer;
use instances::Instances;
use mesh::Meshes;
use shaders::Shaders;
use skin::Skins;
use transform::Transforms;

pub mod bind_groups;
pub mod bounds;
pub mod buffer;
pub mod camera;
pub mod error;
pub mod instances;
pub mod mesh;
pub mod render;
pub mod shaders;
pub mod skin;
pub mod transform;
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
    pub bind_groups: BindGroups,
    pub meshes: Meshes,
    pub camera: CameraBuffer,
    pub transforms: Transforms,
    pub skins: Skins,
    pub instances: Instances,
    pub shaders: Shaders,
    pub logging: AwsmRendererLogging,

    #[cfg(feature = "gltf")]
    gltf: gltf::cache::GltfCache,

    #[cfg(feature = "animation")]
    pub animations: animation::Animations,
}

impl AwsmRenderer {
    pub fn remove_all(&mut self) -> crate::error::Result<()> {
        let deps = RebuildDeps::new(&self.gpu, self.logging.clone())?;
        let RebuildDeps {
            bind_groups,
            meshes,
            camera,
            transforms,
            skins,
            ..
        } = deps;

        self.bind_groups = bind_groups;
        self.camera = camera;
        self.meshes = meshes;
        self.transforms = transforms;
        self.skins = skins;

        #[cfg(feature = "gltf")]
        {
            self.gltf = deps.gltf;
        }

        #[cfg(feature = "animation")]
        {
            self.animations = deps.animations;
        }

        Ok(())
    }
}

pub struct AwsmRendererBuilder {
    gpu: core::renderer::AwsmRendererWebGpuBuilder,
    logging: AwsmRendererLogging,
}

impl AwsmRendererBuilder {
    pub fn new(gpu: web_sys::Gpu) -> Self {
        Self {
            gpu: core::renderer::AwsmRendererWebGpuBuilder::new(gpu),
            logging: AwsmRendererLogging::default(),
        }
    }

    pub fn with_logging(mut self, logging: AwsmRendererLogging) -> Self {
        self.logging = logging;
        self
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

        let deps = RebuildDeps::new(&gpu, self.logging)?;

        Ok(AwsmRenderer {
            gpu,
            meshes: deps.meshes,
            camera: deps.camera,
            transforms: deps.transforms,
            skins: deps.skins,
            instances: deps.instances,
            shaders: deps.shaders,
            bind_groups: deps.bind_groups,
            logging: deps.logging,

            #[cfg(feature = "gltf")]
            gltf: deps.gltf,

            #[cfg(feature = "animation")]
            animations: deps.animations,
        })
    }
}

struct RebuildDeps {
    pub bind_groups: BindGroups,
    pub meshes: Meshes,
    pub camera: CameraBuffer,
    pub transforms: Transforms,
    pub skins: Skins,
    pub instances: Instances,
    pub shaders: Shaders,
    pub logging: AwsmRendererLogging,

    #[cfg(feature = "gltf")]
    pub gltf: gltf::cache::GltfCache,

    #[cfg(feature = "animation")]
    pub animations: animation::Animations,
}

impl RebuildDeps {
    pub fn new(
        gpu: &AwsmRendererWebGpu,
        logging: AwsmRendererLogging,
    ) -> std::result::Result<Self, crate::error::AwsmError> {
        let bind_groups = bind_groups::BindGroups::new(gpu)?;
        let camera = camera::CameraBuffer::new()?;
        let meshes = Meshes::new(gpu)?;
        let transforms = Transforms::new()?;
        let skins = Skins::new();
        let instances = Instances::new(gpu)?;
        let shaders = Shaders::new();

        Ok(Self {
            bind_groups,
            meshes,
            camera,
            transforms,
            skins,
            logging,
            instances,
            shaders,

            #[cfg(feature = "gltf")]
            gltf: gltf::cache::GltfCache::default(),
            #[cfg(feature = "animation")]
            animations: animation::Animations::default(),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct AwsmRendererLogging {
    pub render_timings: bool,
}
