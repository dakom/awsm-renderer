use awsm_renderer_core::command::{render_pass::{ColorAttachment, DepthStencilAttachment, RenderPassDescriptor}, LoadOp, StoreOp};

use crate::{render::RenderContext, AwsmRenderer, error::Result};

impl AwsmRenderer {
    pub(crate) fn render_geometry_pass(&self, ctx: &RenderContext) -> Result<()> {
        let _maybe_span_guard = if self.logging.render_timings {
            Some(tracing::span!(tracing::Level::INFO, "Render Geometry Pass").entered())
        } else {
            None
        };

        let renderables = self.collect_renderables(false);

        let render_pass = ctx.command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![
                    ColorAttachment::new(&ctx.texture_views.entity_id, LoadOp::Clear, StoreOp::Store),
                    ColorAttachment::new(&ctx.texture_views.world_normal, LoadOp::Clear, StoreOp::Store),
                    ColorAttachment::new(&ctx.texture_views.curr_screen_pos, LoadOp::Clear, StoreOp::Store),
                    ColorAttachment::new(&ctx.texture_views.motion_vector, LoadOp::Clear, StoreOp::Store),
                ],
                depth_stencil_attachment: Some(
                    DepthStencilAttachment::new(&ctx.texture_views.depth)
                        .with_depth_load_op(LoadOp::Clear)
                        .with_depth_store_op(StoreOp::Store)
                        .with_depth_clear_value(1.0),
                ),
                ..Default::default()
            }
            .into(),
        )?;

        render_pass.set_bind_group(
            0,
            ctx.bind_groups.render_pass.geometry.camera_lights.get_bind_group()?,
            None,
        )?;

        let mut last_render_pipeline_key = None;
        for renderable in renderables {
            let render_pipeline_key = renderable.render_pipeline_key();
            if last_render_pipeline_key != Some(render_pipeline_key) {
                render_pass
                    .set_pipeline(ctx.pipelines.render.get(render_pipeline_key)?);
                last_render_pipeline_key = Some(render_pipeline_key);
            }

            renderable.push_commands(ctx, &render_pass)?;
        }

        render_pass.end();

        Ok(())
    }

}