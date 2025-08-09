use awsm_renderer_core::{renderer::AwsmRendererWebGpu, texture::TextureFormat};

use crate::error::Result;
use crate::{
    bind_group_layout::BindGroupLayouts,
    bind_groups::BindGroups,
    pipeline_layouts::PipelineLayouts,
    pipelines::Pipelines,
    render_passes::{
        composite::render_pass::CompositeRenderPass,
        display::render_pass::DisplayRenderPass,
        geometry::render_pass::GeometryRenderPass,
        light_culling::render_pass::LightCullingRenderPass,
        material::{
            opaque::render_pass::MaterialOpaqueRenderPass,
            transparent::render_pass::MaterialTransparentRenderPass,
        },
    },
    render_textures::{RenderTextureFormats, RenderTextureViews},
    shaders::Shaders,
    textures::Textures,
};

pub mod composite;
pub mod display;
pub mod geometry;
pub mod light_culling;
pub mod material;
pub mod shader_cache_key;
pub mod shader_template;

pub struct RenderPasses {
    pub geometry: GeometryRenderPass,
    pub light_culling: LightCullingRenderPass,
    pub material_opaque: MaterialOpaqueRenderPass,
    pub material_transparent: MaterialTransparentRenderPass,
    pub composite: CompositeRenderPass,
    pub display: DisplayRenderPass,
}

impl RenderPasses {
    pub async fn new<'a>(ctx: &mut RenderPassInitContext<'a>) -> Result<Self> {
        Ok(Self {
            geometry: GeometryRenderPass::new(ctx).await?,
            light_culling: LightCullingRenderPass::new(ctx).await?,
            material_opaque: MaterialOpaqueRenderPass::new(ctx).await?,
            material_transparent: MaterialTransparentRenderPass::new(ctx).await?,
            composite: CompositeRenderPass::new(ctx).await?,
            display: DisplayRenderPass::new(ctx).await?,
        })
    }

    pub async fn update_texture_bindings(
        &mut self,
        ctx: &mut RenderPassInitContext<'_>,
    ) -> Result<()> {
        self.material_opaque.update_texture_bindings(ctx).await?;
        self.material_transparent
            .update_texture_bindings(ctx)
            .await?;
        Ok(())
    }
}

pub struct RenderPassInitContext<'a> {
    pub gpu: &'a mut AwsmRendererWebGpu,
    pub bind_group_layouts: &'a mut BindGroupLayouts,
    pub textures: &'a mut Textures,
    pub pipeline_layouts: &'a mut PipelineLayouts,
    pub pipelines: &'a mut Pipelines,
    pub shaders: &'a mut Shaders,
    pub render_texture_formats: &'a mut RenderTextureFormats,
}
