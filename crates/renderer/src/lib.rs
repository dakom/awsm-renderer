use awsm_renderer_core::{
    command::color::Color,
    renderer::{AwsmRendererWebGpu, AwsmRendererWebGpuBuilder},
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
    textures::{RenderTextureFormats, RenderTextures},
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
    pub render_textures: RenderTextures,
    pub post_process: PostProcess,
    // we pick between these on the fly
    _clear_color_perceptual_to_linear: Color,
    _clear_color: Color,

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
            .with_clear_color(self._clear_color.clone())
            .with_render_texture_formats(self.render_textures.formats.clone())
            .build()
            .await?;

        *self = renderer;
        Ok(())
    }
}

pub struct AwsmRendererBuilder {
    gpu: AwsmRendererGpuBuilderKind,
    logging: AwsmRendererLogging,
    render_texture_formats: RenderTextureFormats,
    clear_color: Color,
    post_process_settings: PostProcessSettings,
}

pub enum AwsmRendererGpuBuilderKind {
    WebGpuBuilder(AwsmRendererWebGpuBuilder),
    WebGpuBuilt(AwsmRendererWebGpu),
}

impl From<AwsmRendererWebGpuBuilder> for AwsmRendererGpuBuilderKind {
    fn from(builder: AwsmRendererWebGpuBuilder) -> Self {
        AwsmRendererGpuBuilderKind::WebGpuBuilder(builder)
    }
}

impl From<AwsmRendererWebGpu> for AwsmRendererGpuBuilderKind {
    fn from(gpu: AwsmRendererWebGpu) -> Self {
        AwsmRendererGpuBuilderKind::WebGpuBuilt(gpu)
    }
}

impl AwsmRendererBuilder {
    pub fn new(gpu: impl Into<AwsmRendererGpuBuilderKind>) -> Self {
        Self {
            gpu: gpu.into(),
            logging: AwsmRendererLogging::default(),
            render_texture_formats: RenderTextureFormats::default(),
            clear_color: Color::BLACK,
            post_process_settings: PostProcessSettings::default(),
        }
    }

    pub fn with_logging(mut self, logging: AwsmRendererLogging) -> Self {
        self.logging = logging;
        self
    }

    pub fn with_post_process(mut self, settings: PostProcessSettings) -> Self {
        self.post_process_settings = settings;
        self
    }

    pub fn with_render_texture_formats(mut self, formats: RenderTextureFormats) -> Self {
        self.render_texture_formats = formats;
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
            render_texture_formats,
            clear_color,
            post_process_settings,
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
        let render_textures = RenderTextures::new(render_texture_formats);
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
            _clear_color: clear_color.clone(),
            _clear_color_perceptual_to_linear: clear_color.perceptual_to_linear(),
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
