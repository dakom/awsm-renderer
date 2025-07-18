#[allow(clippy::uninlined_format_args)]
use awsm_renderer_core::{
    command::color::Color,
    renderer::{AwsmRendererWebGpu, AwsmRendererWebGpuBuilder},
    texture::TextureFormat,
};
use bind_groups::BindGroups;
use camera::CameraBuffer;
use instances::Instances;
use lights::Lights;
use materials::Materials;
use mesh::Meshes;
use pipeline::Pipelines;
use shaders::Shaders;
use skin::Skins;
use textures::Textures;
use transform::Transforms;

use crate::render::{
    post_process::{PostProcess, PostProcessSettings},
    textures::RenderTextures,
};

pub mod bind_groups;
pub mod bounds;
pub mod buffer;
pub mod camera;
pub mod error;
pub mod instances;
pub mod lights;
pub mod materials;
pub mod mesh;
pub mod pipeline;
pub mod render;
pub mod renderable;
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
    pub pipelines: Pipelines,
    pub lights: Lights,
    pub textures: Textures,
    pub logging: AwsmRendererLogging,
    pub clear_color: Color,
    pub render_textures: RenderTextures,
    pub post_process: PostProcess,

    #[cfg(feature = "gltf")]
    gltf: gltf::cache::GltfCache,

    #[cfg(feature = "animation")]
    pub animations: animation::Animations,
}

impl AwsmRenderer {
    pub async fn remove_all(&mut self) -> crate::error::Result<()> {
        // meh, just recreate the renderer, it's fine
        let renderer = AwsmRendererBuilder::new(self.gpu.clone())
            .with_logging(self.logging.clone())
            .with_clear_color(self.clear_color.clone())
            .with_scene_texture_format(self.render_textures.scene_texture_format)
            .with_depth_texture_format(self.render_textures.depth_texture_format)
            .build()
            .await?;

        *self = renderer;
        Ok(())
    }
}

pub struct AwsmRendererBuilder<'a> {
    gpu: AwsmRendererGpuBuilderKind<'a>,
    logging: AwsmRendererLogging,
    scene_texture_format: TextureFormat,
    depth_texture_format: TextureFormat,
    clear_color: Color,
}

pub enum AwsmRendererGpuBuilderKind<'a> {
    WebGpuBuilder(AwsmRendererWebGpuBuilder<'a>),
    WebGpuBuilt(AwsmRendererWebGpu),
}

impl<'a> From<AwsmRendererWebGpuBuilder<'a>> for AwsmRendererGpuBuilderKind<'a> {
    fn from(builder: AwsmRendererWebGpuBuilder<'a>) -> Self {
        AwsmRendererGpuBuilderKind::WebGpuBuilder(builder)
    }
}

impl From<AwsmRendererWebGpu> for AwsmRendererGpuBuilderKind<'_> {
    fn from(gpu: AwsmRendererWebGpu) -> Self {
        AwsmRendererGpuBuilderKind::WebGpuBuilt(gpu)
    }
}

impl From<(web_sys::Gpu, web_sys::HtmlCanvasElement)> for AwsmRendererGpuBuilderKind<'_> {
    fn from((gpu, canvas): (web_sys::Gpu, web_sys::HtmlCanvasElement)) -> Self {
        AwsmRendererGpuBuilderKind::WebGpuBuilder(AwsmRendererWebGpuBuilder::new(gpu, canvas))
    }
}

impl<'a> AwsmRendererBuilder<'a> {
    pub fn new(gpu: impl Into<AwsmRendererGpuBuilderKind<'a>>) -> Self {
        Self {
            gpu: gpu.into(),
            logging: AwsmRendererLogging::default(),
            scene_texture_format: TextureFormat::Rgba16float, // HDR format for bloom/tonemapping
            depth_texture_format: TextureFormat::Depth24plus,
            clear_color: Color::BLACK,
        }
    }

    pub fn with_logging(mut self, logging: AwsmRendererLogging) -> Self {
        self.logging = logging;
        self
    }

    pub fn with_scene_texture_format(mut self, format: TextureFormat) -> Self {
        self.scene_texture_format = format;
        self
    }

    pub fn with_depth_texture_format(mut self, format: TextureFormat) -> Self {
        self.depth_texture_format = format;
        self
    }

    pub fn with_clear_color(mut self, color: Color) -> Self {
        self.clear_color = color;
        self
    }

    pub async fn build(self) -> std::result::Result<AwsmRenderer, crate::error::AwsmError> {
        let Self {
            gpu,
            logging,
            scene_texture_format,
            depth_texture_format,
            clear_color,
        } = self;

        let gpu = match gpu {
            AwsmRendererGpuBuilderKind::WebGpuBuilder(builder) => builder.build().await?,
            AwsmRendererGpuBuilderKind::WebGpuBuilt(gpu) => gpu,
        };
        let bind_groups = bind_groups::BindGroups::new(&gpu)?;
        let camera = camera::CameraBuffer::new()?;
        let meshes = Meshes::new(&gpu)?;
        let transforms = Transforms::new()?;
        let skins = Skins::new();
        let instances = Instances::new(&gpu)?;
        let shaders = Shaders::new();
        let materials = Materials::new();
        let pipelines = Pipelines::new();
        let lights = Lights::new();
        let textures = Textures::new();
        let post_process_settings = PostProcessSettings::default();
        let render_textures = RenderTextures::new(scene_texture_format, depth_texture_format);
        let post_process = PostProcess::new(post_process_settings);
        #[cfg(feature = "gltf")]
        let gltf = gltf::cache::GltfCache::default();
        #[cfg(feature = "animation")]
        let animations = animation::Animations::default();

        let mut _self = AwsmRenderer {
            gpu,
            meshes,
            camera,
            transforms,
            skins,
            instances,
            shaders,
            bind_groups,
            materials,
            pipelines,
            lights,
            textures,
            clear_color,
            logging,
            render_textures,
            post_process,
            #[cfg(feature = "gltf")]
            gltf,
            #[cfg(feature = "animation")]
            animations,
        };

        _self.post_process_init().await?;

        Ok(_self)
    }
}

#[derive(Clone, Debug, Default)]
pub struct AwsmRendererLogging {
    pub render_timings: bool,
}
