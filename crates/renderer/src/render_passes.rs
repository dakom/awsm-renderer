use awsm_renderer_core::renderer::AwsmRendererWebGpu;

use crate::{bind_group_layout::BindGroupLayouts, bind_groups::BindGroups, pipeline_layouts::PipelineLayouts, pipelines::Pipelines, render_passes::{composite::render_pass::CompositeRenderPass, display::render_pass::DisplayRenderPass, geometry::render_pass::GeometryRenderPass, light_culling::render_pass::LightCullingRenderPass, material::{opaque::render_pass::MaterialOpaqueRenderPass, transparent::render_pass::MaterialTransparentRenderPass}}, render_textures::{RenderTextureFormats, RenderTextureViews}, shaders::Shaders, textures::Textures};
use crate::error::Result;

pub mod geometry;
pub mod light_culling;
pub mod material;
pub mod composite;
pub mod display;
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
    pub async fn new(ctx: &mut RenderPassInitContext) -> Result<Self> {
        Ok(Self {
            geometry: GeometryRenderPass::new(ctx).await?,
            light_culling: LightCullingRenderPass::new(ctx).await?,
            material_opaque: MaterialOpaqueRenderPass::new(ctx).await?,
            material_transparent: MaterialTransparentRenderPass::new(ctx).await?,
            composite: CompositeRenderPass::new(ctx).await?,
            display: DisplayRenderPass::new(ctx).await?,
        })
    }
}

pub struct RenderPassInitContext {
    pub gpu: AwsmRendererWebGpu, 
    pub bind_group_layouts: BindGroupLayouts, 
    pub textures: Textures,
    pub pipeline_layouts: PipelineLayouts,
    pub pipelines: Pipelines,
    pub shaders: Shaders,
    pub render_texture_formats: RenderTextureFormats,
}