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
    render_passes::{
        display::{bind_group::DisplayBindGroups, shader::cache_key::ShaderCacheKeyDisplay},
        RenderPassInitContext,
    },
    render_textures::RenderTextureFormats,
    shaders::Shaders,
};

pub struct DisplayPipelines {
    pub pipeline_layout_key: PipelineLayoutKey,
    pub smaa_render_pipeline_key: RenderPipelineKey,
    pub no_anti_alias_render_pipeline_key: RenderPipelineKey,
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

        let smaa_render_pipeline_key = init_pipeline_key(
            pipeline_layout_key.clone(),
            true,
            &ctx.gpu,
            &mut ctx.shaders,
            &mut ctx.pipelines,
            &ctx.pipeline_layouts,
            &ctx.render_texture_formats,
        )
        .await?;

        let no_anti_alias_render_pipeline_key = init_pipeline_key(
            pipeline_layout_key.clone(),
            false,
            &ctx.gpu,
            &mut ctx.shaders,
            &mut ctx.pipelines,
            &ctx.pipeline_layouts,
            &ctx.render_texture_formats,
        )
        .await?;

        Ok(Self {
            pipeline_layout_key,
            smaa_render_pipeline_key,
            no_anti_alias_render_pipeline_key,
        })
    }
}

async fn init_pipeline_key(
    pipeline_layout_key: PipelineLayoutKey,
    smaa_anti_alias: bool,
    gpu: &AwsmRendererWebGpu,
    shaders: &mut Shaders,
    pipelines: &mut Pipelines,
    pipeline_layouts: &PipelineLayouts,
    render_texture_formats: &RenderTextureFormats,
) -> Result<RenderPipelineKey> {
    let shader_cache_key = ShaderCacheKeyDisplay { smaa_anti_alias };
    let shader_key = shaders.get_key(&gpu, shader_cache_key).await?;

    let render_pipeline_cache_key = RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
        .with_push_fragment_target(ColorTargetState::new(gpu.current_context_format()))
        .with_primitive(
            PrimitiveState::new()
                .with_topology(web_sys::GpuPrimitiveTopology::TriangleList)
                .with_cull_mode(web_sys::GpuCullMode::None)
                .with_front_face(web_sys::GpuFrontFace::Ccw),
        );

    Ok(pipelines
        .render
        .get_key(&gpu, &shaders, &pipeline_layouts, render_pipeline_cache_key)
        .await?)
}
