use awsm_renderer_core::{
    pipeline::{fragment::ColorTargetState, primitive::PrimitiveState},
    renderer::AwsmRendererWebGpu,
};

use crate::{
    error::Result,
    pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey, PipelineLayouts},
    pipelines::{
        render_pipeline::{RenderPipelineCacheKey, RenderPipelineKey},
        Pipelines,
    },
    post_process::PostProcessing,
    render_passes::{
        display::{bind_group::DisplayBindGroups, shader::cache_key::ShaderCacheKeyDisplay},
        RenderPassInitContext,
    },
    render_textures::RenderTextureFormats,
    shaders::Shaders,
};

pub struct DisplayPipelines {
    pub pipeline_layout_key: PipelineLayoutKey,
    pub render_pipeline_key: Option<RenderPipelineKey>,
}

impl DisplayPipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &DisplayBindGroups,
    ) -> Result<Self> {
        let pipeline_layout_cache_key =
            PipelineLayoutCacheKey::new(vec![bind_groups.bind_group_layout_key]);

        let pipeline_layout_key = ctx.pipeline_layouts.get_key(
            ctx.gpu,
            ctx.bind_group_layouts,
            pipeline_layout_cache_key,
        )?;

        Ok(Self {
            pipeline_layout_key,
            render_pipeline_key: None,
        })
    }

    pub async fn set_render_pipeline_key(
        &mut self,
        post_processing: &PostProcessing,
        gpu: &AwsmRendererWebGpu,
        shaders: &mut Shaders,
        pipelines: &mut Pipelines,
        pipeline_layouts: &PipelineLayouts,
        _render_texture_formats: &RenderTextureFormats,
    ) -> Result<()> {
        let shader_cache_key = ShaderCacheKeyDisplay {
            tonemapping: post_processing.tonemapping,
        };
        let shader_key = shaders.get_key(gpu, shader_cache_key).await?;

        let render_pipeline_cache_key =
            RenderPipelineCacheKey::new(shader_key, self.pipeline_layout_key)
                .with_push_fragment_target(ColorTargetState::new(gpu.current_context_format()))
                .with_primitive(
                    PrimitiveState::new()
                        .with_topology(web_sys::GpuPrimitiveTopology::TriangleList)
                        .with_cull_mode(web_sys::GpuCullMode::None)
                        .with_front_face(web_sys::GpuFrontFace::Ccw),
                );

        self.render_pipeline_key = Some(
            pipelines
                .render
                .get_key(gpu, shaders, pipeline_layouts, render_pipeline_cache_key)
                .await?,
        );

        Ok(())
    }
}
