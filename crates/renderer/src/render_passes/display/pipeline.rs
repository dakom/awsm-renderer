use awsm_renderer_core::pipeline::{fragment::ColorTargetState, primitive::PrimitiveState};

use crate::{
    error::Result,
    pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey},
    pipelines::render_pipeline::{RenderPipelineCacheKey, RenderPipelineKey},
    render_passes::{
        display::{bind_group::DisplayBindGroups, shader::cache_key::ShaderCacheKeyDisplay},
        RenderPassInitContext,
    },
};

pub struct DisplayPipelines {
    pub pipeline_layout_key: PipelineLayoutKey,
    pub render_pipeline_key: RenderPipelineKey,
}

impl DisplayPipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &DisplayBindGroups,
    ) -> Result<Self> {
        let pipeline_layout_cache_key =
            PipelineLayoutCacheKey::new(vec![bind_groups.bind_group_layout_key.clone()]);
        let pipeline_layout_key = ctx.pipeline_layouts.get_key(
            &ctx.gpu,
            &ctx.bind_group_layouts,
            pipeline_layout_cache_key,
        )?;

        let shader_cache_key = ShaderCacheKeyDisplay {};
        let shader_key = ctx.shaders.get_key(&ctx.gpu, shader_cache_key).await?;

        let render_pipeline_cache_key =
            RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
                .with_push_fragment_target(ColorTargetState::new(ctx.gpu.current_context_format()))
                .with_primitive(
                    PrimitiveState::new()
                        .with_topology(web_sys::GpuPrimitiveTopology::TriangleList)
                        .with_cull_mode(web_sys::GpuCullMode::None)
                        .with_front_face(web_sys::GpuFrontFace::Ccw),
                );
        let render_pipeline_key = ctx
            .pipelines
            .render
            .get_key(
                &ctx.gpu,
                &ctx.shaders,
                &ctx.pipeline_layouts,
                render_pipeline_cache_key,
            )
            .await?;

        Ok(Self {
            pipeline_layout_key,
            render_pipeline_key,
        })
    }
}
