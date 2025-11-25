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
    pub multisampled_compute_pipeline_key: ComputePipelineKey,
    pub singlesampled_compute_pipeline_key: ComputePipelineKey,
}

impl CompositePipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &CompositeBindGroups,
    ) -> Result<Self> {
        let multisampled_pipeline_layout_cache_key = PipelineLayoutCacheKey::new(vec![bind_groups
            .multisampled_bind_group_layout_key
            .clone()]);
        let singlesampled_pipeline_layout_cache_key =
            PipelineLayoutCacheKey::new(vec![bind_groups
                .singlesampled_bind_group_layout_key
                .clone()]);

        let multisampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            &ctx.gpu,
            &ctx.bind_group_layouts,
            multisampled_pipeline_layout_cache_key,
        )?;
        let singlesampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            &ctx.gpu,
            &ctx.bind_group_layouts,
            singlesampled_pipeline_layout_cache_key,
        )?;

        let multisampled_shader_cache_key = ShaderCacheKeyComposite {
            multisampled_geometry: true,
        };
        let multisampled_shader_key = ctx
            .shaders
            .get_key(&ctx.gpu, multisampled_shader_cache_key)
            .await?;

        let singlesampled_shader_cache_key = ShaderCacheKeyComposite {
            multisampled_geometry: false,
        };
        let singlesampled_shader_key = ctx
            .shaders
            .get_key(&ctx.gpu, singlesampled_shader_cache_key)
            .await?;

        let multisampled_compute_pipeline_cache_key =
            ComputePipelineCacheKey::new(multisampled_shader_key, multisampled_pipeline_layout_key);

        let singlesampled_compute_pipeline_cache_key = ComputePipelineCacheKey::new(
            singlesampled_shader_key,
            singlesampled_pipeline_layout_key,
        );

        let multisampled_compute_pipeline_key = ctx
            .pipelines
            .compute
            .get_key(
                &ctx.gpu,
                &ctx.shaders,
                &ctx.pipeline_layouts,
                multisampled_compute_pipeline_cache_key,
            )
            .await?;

        let singlesampled_compute_pipeline_key = ctx
            .pipelines
            .compute
            .get_key(
                &ctx.gpu,
                &ctx.shaders,
                &ctx.pipeline_layouts,
                singlesampled_compute_pipeline_cache_key,
            )
            .await?;

        Ok(Self {
            multisampled_compute_pipeline_key,
            singlesampled_compute_pipeline_key,
        })
    }
}
