#![allow(warnings)]

pub mod bind_groups;
pub mod bind_group_layout;
pub mod bounds;
pub mod buffer;
pub mod camera;
pub mod error;
pub mod instances;
pub mod lights;
pub mod materials;
pub mod mesh;
pub mod pipelines;
pub mod pipeline_layouts;
pub mod render;
pub mod render_textures;
pub mod render_passes;
pub mod renderable;
pub mod shaders;
pub mod textures;
pub mod transforms;
pub mod update;
pub mod core {
    pub use awsm_renderer_core::*;
}
#[cfg(feature = "gltf")]
pub mod gltf;

#[cfg(feature = "animation")]
pub mod animation;

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
use pipelines::Pipelines;
use shaders::Shaders;
use mesh::skins::Skins;
use textures::Textures;
use transforms::Transforms;

use crate::{bind_group_layout::BindGroupLayouts, pipeline_layouts::PipelineLayouts, render_passes::{geometry::bind_group::GeometryBindGroups, RenderPassInitContext, RenderPasses}, render_textures::{RenderTextureFormats, RenderTextures}};


pub struct AwsmRenderer {
    pub gpu: core::renderer::AwsmRendererWebGpu,
    pub bind_group_layouts: BindGroupLayouts,
    pub bind_groups: BindGroups,
    pub meshes: Meshes,
    pub camera: CameraBuffer,
    pub transforms: Transforms,
    pub instances: Instances,
    pub shaders: Shaders,
    pub materials: Materials,
    pub pipeline_layouts: PipelineLayouts,
    pub pipelines: Pipelines,
    pub lights: Lights,
    pub textures: Textures,
    pub logging: AwsmRendererLogging,
    pub render_textures: RenderTextures,
    pub render_passes: RenderPasses,
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
        }
    }

    pub fn with_logging(mut self, logging: AwsmRendererLogging) -> Self {
        self.logging = logging;
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
        } = self;

        let gpu = match gpu {
            AwsmRendererGpuBuilderKind::WebGpuBuilder(builder) => builder.build().await?,
            AwsmRendererGpuBuilderKind::WebGpuBuilt(gpu) => gpu,
        };

        let mut pipeline_layouts = PipelineLayouts::new();
        let mut bind_group_layouts = BindGroupLayouts::new();
        let mut pipelines = Pipelines::new();
        let mut shaders = Shaders::new();
        let textures = Textures::new();

        let camera = camera::CameraBuffer::new(&gpu)?;
        let lights = Lights::new(&gpu)?;
        let meshes = Meshes::new(&gpu)?;
        let transforms = Transforms::new(&gpu)?;
        let instances = Instances::new(&gpu)?;
        let materials = Materials::new(&gpu)?;

        // temporarily push into an init struct for creating render passes
        // we'll then destructure it to get our values back
        let mut render_pass_init = RenderPassInitContext {
            gpu,
            bind_group_layouts,
            pipeline_layouts,
            pipelines,
            shaders,
            render_texture_formats,
            textures,
        };
        let render_passes = RenderPasses::new(&mut render_pass_init).await?;
        let RenderPassInitContext { gpu, bind_group_layouts, pipeline_layouts, pipelines, shaders, render_texture_formats, textures} = render_pass_init;

        let bind_groups = BindGroups::new();
        let render_textures = RenderTextures::new(render_texture_formats);
        #[cfg(feature = "gltf")]
        let gltf = gltf::cache::GltfCache::default();
        #[cfg(feature = "animation")]
        let animations = animation::Animations::default();

        let mut _self = AwsmRenderer {
            gpu,
            meshes,
            camera,
            transforms,
            instances,
            shaders,
            bind_group_layouts,
            bind_groups,
            materials,
            pipeline_layouts,
            pipelines,
            lights,
            textures,
            render_passes,
            _clear_color: clear_color.clone(),
            _clear_color_perceptual_to_linear: clear_color.perceptual_to_linear(),
            logging,
            render_textures,
            #[cfg(feature = "gltf")]
            gltf,
            #[cfg(feature = "animation")]
            animations,
        };

        Ok(_self)
    }
}

#[derive(Clone, Debug, Default)]
pub struct AwsmRendererLogging {
    pub render_timings: bool,
}
