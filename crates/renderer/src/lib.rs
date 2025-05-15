use awsm_renderer_core::{
    command::color::Color,
    configuration::CanvasConfiguration,
    renderer::AwsmRendererWebGpu,
    texture::{Extent3d, TextureDescriptor, TextureFormat, TextureUsage},
};
use bind_groups::BindGroups;
use camera::CameraBuffer;
use instances::Instances;
use lights::Lights;
use materials::Materials;
use mesh::Meshes;
use shaders::Shaders;
use skin::Skins;
use textures::Textures;
use transform::Transforms;

pub mod bind_groups;
pub mod bounds;
pub mod buffer;
pub mod camera;
pub mod error;
pub mod instances;
pub mod lights;
pub mod materials;
pub mod mesh;
pub mod render;
pub mod shaders;
pub mod skin;
pub mod textures;
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
    pub materials: Materials,
    pub lights: Lights,
    pub textures: Textures,
    pub logging: AwsmRendererLogging,
    pub clear_color: Color,
    pub depth_texture: Option<web_sys::GpuTexture>,

    #[cfg(feature = "gltf")]
    gltf: gltf::cache::GltfCache,

    #[cfg(feature = "animation")]
    pub animations: animation::Animations,
}

impl AwsmRenderer {
    pub fn remove_all(&mut self) -> crate::error::Result<()> {
        let deps = RebuildDeps::new(&self.gpu)?;
        let RebuildDeps {
            bind_groups,
            meshes,
            camera,
            transforms,
            skins,
            instances,
            shaders,
            materials,
            textures,
            lights,
            #[cfg(feature = "gltf")]
            gltf,
            #[cfg(feature = "animation")]
            animations,
        } = deps;

        self.bind_groups = bind_groups;
        self.camera = camera;
        self.meshes = meshes;
        self.transforms = transforms;
        self.skins = skins;
        self.instances = instances;
        self.shaders = shaders;
        self.materials = materials;
        self.lights = lights;
        self.textures = textures;
        // nah... keep this, application logic can reset it
        // self.depth_texture = None;

        #[cfg(feature = "gltf")]
        {
            self.gltf = gltf;
        }

        #[cfg(feature = "animation")]
        {
            self.animations = animations;
        }

        Ok(())
    }

    pub fn set_depth_texture(
        &mut self,
        format: TextureFormat,
        width: u32,
        height: u32,
    ) -> crate::error::Result<()> {
        let depth_texture = self.gpu.create_texture(
            &TextureDescriptor::new(
                format,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new().with_render_attachment(),
            )
            .into(),
        )?;

        self.depth_texture = Some(depth_texture);

        Ok(())
    }
    pub fn clear_depth_texture(&mut self) -> crate::error::Result<()> {
        if let Some(texture) = self.depth_texture.take() {
            texture.destroy();
        }
        Ok(())
    }
}

pub struct AwsmRendererBuilder {
    gpu: core::renderer::AwsmRendererWebGpuBuilder,
    logging: AwsmRendererLogging,
    depth_texture: Option<web_sys::GpuTexture>,
}

impl AwsmRendererBuilder {
    pub fn new(gpu: web_sys::Gpu) -> Self {
        Self {
            gpu: core::renderer::AwsmRendererWebGpuBuilder::new(gpu),
            logging: AwsmRendererLogging::default(),
            depth_texture: None,
        }
    }

    pub fn with_logging(mut self, logging: AwsmRendererLogging) -> Self {
        self.logging = logging;
        self
    }

    pub fn with_depth_texture(mut self, texture: web_sys::GpuTexture) -> Self {
        self.depth_texture = Some(texture);
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

    pub fn init_context(
        mut self,
        canvas: web_sys::HtmlCanvasElement,
        configuration: Option<CanvasConfiguration>,
    ) -> core::error::Result<Self> {
        self.gpu = self.gpu.init_context(canvas, configuration)?;
        Ok(self)
    }

    pub fn build(self) -> std::result::Result<AwsmRenderer, crate::error::AwsmError> {
        let gpu = self.gpu.build()?;

        let deps = RebuildDeps::new(&gpu)?;

        Ok(AwsmRenderer {
            gpu,
            meshes: deps.meshes,
            camera: deps.camera,
            transforms: deps.transforms,
            skins: deps.skins,
            instances: deps.instances,
            shaders: deps.shaders,
            bind_groups: deps.bind_groups,
            materials: deps.materials,
            lights: deps.lights,
            textures: deps.textures,
            clear_color: Color::BLACK,
            logging: self.logging,
            depth_texture: self.depth_texture,

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
    pub materials: Materials,
    pub lights: Lights,
    pub textures: Textures,

    #[cfg(feature = "gltf")]
    pub gltf: gltf::cache::GltfCache,

    #[cfg(feature = "animation")]
    pub animations: animation::Animations,
}

impl RebuildDeps {
    pub fn new(gpu: &AwsmRendererWebGpu) -> std::result::Result<Self, crate::error::AwsmError> {
        let bind_groups = bind_groups::BindGroups::new(gpu)?;
        let camera = camera::CameraBuffer::new()?;
        let meshes = Meshes::new(gpu)?;
        let transforms = Transforms::new()?;
        let skins = Skins::new();
        let instances = Instances::new(gpu)?;
        let shaders = Shaders::new();
        let materials = Materials::new();
        let lights = Lights::new();
        let textures = Textures::new();

        Ok(Self {
            bind_groups,
            meshes,
            camera,
            transforms,
            skins,
            instances,
            shaders,
            materials,
            lights,
            textures,

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
