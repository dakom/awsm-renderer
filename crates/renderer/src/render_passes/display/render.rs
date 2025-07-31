use awsm_renderer_core::command::{render_pass::{ColorAttachment, RenderPassDescriptor}, LoadOp, StoreOp};

use crate::{render::RenderContext, AwsmRenderer, error::Result};

impl AwsmRenderer {
    pub(crate) fn render_display_pass(&self, ctx: &RenderContext) -> Result<()> {
        let _maybe_span_guard = if self.logging.render_timings {
            Some(tracing::span!(tracing::Level::INFO, "Render Display Pass").entered())
        } else {
            None
        };

        let render_pass = ctx.command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![
                    ColorAttachment::new(&self.gpu.current_context_texture_view()?, LoadOp::Clear, StoreOp::Store)
                ],
                ..Default::default()
            }
            .into(),
        )?;

        //render_pass.set_pipeline(ctx.pipelines.render.get(key)

        //render_pass.set_bind_group(0, material_bind_group, None)?;
        // No vertex buffer needed!
        render_pass.draw(3); // Draw 3 vertices to form a triangle

        render_pass.end();

        // TODO!

        Ok(())
    }
}