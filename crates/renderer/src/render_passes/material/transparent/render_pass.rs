use crate::{
    error::Result,
    render::RenderContext,
    render_passes::{
        material::transparent::{
            bind_group::MaterialTransparentBindGroups, pipeline::MaterialTransparentPipelines,
        },
        RenderPassInitContext,
    },
    renderable::{self, Renderable},
    AwsmRenderer,
};
use awsm_renderer_core::{
    command::{
        render_pass::{ColorAttachment, DepthStencilAttachment, RenderPassDescriptor},
        LoadOp, StoreOp,
    },
    renderer::AwsmRendererWebGpu,
};

pub struct MaterialTransparentRenderPass {
    pub bind_groups: MaterialTransparentBindGroups,
    pub pipelines: MaterialTransparentPipelines,
}

impl MaterialTransparentRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_groups = MaterialTransparentBindGroups::new(ctx).await?;
        let pipelines = MaterialTransparentPipelines::new(ctx, &bind_groups).await?;

        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    pub fn render(&self, ctx: &RenderContext, renderables: Vec<Renderable>) -> Result<()> {
        let render_pass = ctx.command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                label: Some("Material Transparent Pass"),
                color_attachments: vec![
                    ColorAttachment::new(
                        &ctx.render_texture_views.oit_alpha,
                        LoadOp::Clear,
                        StoreOp::Store,
                    ),
                    ColorAttachment::new(
                        &ctx.render_texture_views.oit_rgb,
                        LoadOp::Clear,
                        StoreOp::Store,
                    ),
                ],
                depth_stencil_attachment: Some(
                    DepthStencilAttachment::new(&ctx.render_texture_views.depth)
                        .with_depth_load_op(LoadOp::Clear)
                        .with_depth_store_op(StoreOp::Store)
                        .with_depth_clear_value(1.0),
                ),
                ..Default::default()
            }
            .into(),
        )?;

        // TODO!

        render_pass.end();

        Ok(())
    }
}
