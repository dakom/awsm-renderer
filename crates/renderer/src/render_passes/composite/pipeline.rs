use crate::{
    error::Result,
    pipeline_layouts::PipelineLayoutCacheKey,
    pipelines::compute_pipeline::{ComputePipelineCacheKey, ComputePipelineKey},
    render_passes::{
        composite::{bind_group::CompositeBindGroups, shader::cache_key::ShaderCacheKeyComposite},
        RenderPassInitContext,
    },
};

pub struct CompositePipelines {
    pub compute_pipeline_key: ComputePipelineKey,
}

impl CompositePipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &CompositeBindGroups,
    ) -> Result<Self> {
        let pipeline_layout_cache_key =
            PipelineLayoutCacheKey::new(vec![bind_groups.bind_group_layout_key.clone()]);
        let pipeline_layout_key = ctx.pipeline_layouts.get_key(
            &ctx.gpu,
            &ctx.bind_group_layouts,
            pipeline_layout_cache_key,
        )?;

        let shader_cache_key = ShaderCacheKeyComposite {};
        let shader_key = ctx.shaders.get_key(&ctx.gpu, shader_cache_key).await?;

        let compute_pipeline_cache_key =
            ComputePipelineCacheKey::new(shader_key, pipeline_layout_key);

        let compute_pipeline_key = ctx
            .pipelines
            .compute
            .get_key(
                &ctx.gpu,
                &ctx.shaders,
                &ctx.pipeline_layouts,
                compute_pipeline_cache_key,
            )
            .await?;

        Ok(Self {
            compute_pipeline_key,
        })
    }
}
