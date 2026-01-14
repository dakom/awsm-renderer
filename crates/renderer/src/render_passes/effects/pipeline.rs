use awsm_renderer_core::renderer::AwsmRendererWebGpu;

use crate::{
    anti_alias::AntiAliasing,
    error::Result,
    pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey, PipelineLayouts},
    pipelines::{
        compute_pipeline::{ComputePipelineCacheKey, ComputePipelineKey},
        Pipelines,
    },
    post_process::PostProcessing,
    render_passes::{
        effects::{bind_group::EffectsBindGroups, shader::cache_key::ShaderCacheKeyEffects},
        RenderPassInitContext,
    },
    render_textures::RenderTextureFormats,
    shaders::Shaders,
};

pub struct EffectsPipelines {
    pub multisampled_pipeline_layout_key: PipelineLayoutKey,
    pub singlesampled_pipeline_layout_key: PipelineLayoutKey,
    pub compute_pipeline_key: Option<ComputePipelineKey>,
}

impl EffectsPipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &EffectsBindGroups,
    ) -> Result<Self> {
        let singlesampled_pipeline_layout_cache_key =
            PipelineLayoutCacheKey::new(vec![bind_groups.singlesampled_bind_group_layout_key]);
        let multisampled_pipeline_layout_cache_key =
            PipelineLayoutCacheKey::new(vec![bind_groups.multisampled_bind_group_layout_key]);

        let singlesampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            ctx.gpu,
            ctx.bind_group_layouts,
            singlesampled_pipeline_layout_cache_key,
        )?;

        let multisampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            ctx.gpu,
            ctx.bind_group_layouts,
            multisampled_pipeline_layout_cache_key,
        )?;

        Ok(Self {
            multisampled_pipeline_layout_key,
            singlesampled_pipeline_layout_key,
            compute_pipeline_key: None,
        })
    }

    pub async fn set_render_pipeline_key(
        &mut self,
        anti_aliasing: &AntiAliasing,
        post_processing: &PostProcessing,
        gpu: &AwsmRendererWebGpu,
        shaders: &mut Shaders,
        pipelines: &mut Pipelines,
        pipeline_layouts: &PipelineLayouts,
        _render_texture_formats: &RenderTextureFormats,
    ) -> Result<()> {
        let multisampled_geometry = anti_aliasing.has_msaa_checked()?;

        let shader_cache_key = ShaderCacheKeyEffects {
            smaa_anti_alias: anti_aliasing.smaa,
            bloom: post_processing.bloom,
            dof: post_processing.dof,
            multisampled_geometry,
        };
        let shader_key = shaders.get_key(gpu, shader_cache_key).await?;

        let compute_pipeline_cache_key = ComputePipelineCacheKey::new(
            shader_key,
            if multisampled_geometry {
                self.multisampled_pipeline_layout_key
            } else {
                self.singlesampled_pipeline_layout_key
            },
        );

        self.compute_pipeline_key = Some(
            pipelines
                .compute
                .get_key(gpu, shaders, pipeline_layouts, compute_pipeline_cache_key)
                .await?,
        );

        Ok(())
    }
}
