use crate::{render::RenderContext, AwsmRenderer, error::Result};

impl AwsmRenderer {
    pub(crate) fn render_light_culling_pass(&self, ctx: &RenderContext) -> Result<()> {
        let _maybe_span_guard = if self.logging.render_timings {
            Some(tracing::span!(tracing::Level::INFO, "Render Light Culling Pass").entered())
        } else {
            None
        };

        // TODO!

        Ok(())
    }
}