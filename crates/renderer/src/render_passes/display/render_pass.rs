use std::vec;

use awsm_renderer_core::{
    bind_groups::{
        BindGroupLayoutResource, SamplerBindingLayout, SamplerBindingType, TextureBindingLayout,
    },
    command::{
        render_pass::{ColorAttachment, RenderPassDescriptor},
        LoadOp, StoreOp,
    },
    pipeline::{fragment::ColorTargetState, primitive::PrimitiveState},
    renderer::AwsmRendererWebGpu,
    texture::{TextureSampleType, TextureViewDimension},
};

use crate::{
    bind_group_layout::{
        BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry, BindGroupLayoutKey, BindGroupLayouts,
    },
    bind_groups::BindGroups,
    error::Result,
    pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey, PipelineLayouts},
    pipelines::{
        render_pipeline::{self, RenderPipelineCacheKey, RenderPipelineKey},
        Pipelines,
    },
    render::{self, RenderContext},
    render_passes::{
        composite::bind_group,
        display::{
            bind_group::DisplayBindGroups, pipeline::DisplayPipelines,
            shader::cache_key::ShaderCacheKeyDisplay,
        },
        RenderPassInitContext,
    },
    render_textures::RenderTextureViews,
    shaders::Shaders,
    AwsmRenderer,
};

pub struct DisplayRenderPass {
    pub bind_groups: DisplayBindGroups,
    pub pipelines: DisplayPipelines,
}

impl DisplayRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_groups = DisplayBindGroups::new(ctx).await?;
        let pipelines = DisplayPipelines::new(ctx, &bind_groups).await?;

        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    pub fn render(&self, ctx: &RenderContext) -> Result<()> {
        let render_pass = ctx.command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                label: Some("Display Render Pass"),
                color_attachments: vec![ColorAttachment::new(
                    &ctx.gpu.current_context_texture_view()?,
                    LoadOp::Clear,
                    StoreOp::Store,
                )],
                ..Default::default()
            }
            .into(),
        )?;

        render_pass.set_bind_group(0, self.bind_groups.get_bind_group()?, None)?;

        render_pass.set_pipeline(ctx.pipelines.render.get(if ctx.anti_aliasing.smaa {
            self.pipelines.smaa_render_pipeline_key
        } else {
            self.pipelines.no_anti_alias_render_pipeline_key
        })?);

        // No vertex buffer needed!
        render_pass.draw(3);

        render_pass.end();

        // TODO!

        Ok(())
    }
}
