use crate::{render::RenderContext, AwsmRenderer, error::Result};

impl AwsmRenderer {
    pub(crate) fn render_composite_pass(&self, ctx: &RenderContext) -> Result<()> {
        let _maybe_span_guard = if self.logging.render_timings {
            Some(tracing::span!(tracing::Level::INFO, "Render Composite Pass").entered())
        } else {
            None
        };

        // TODO!

        Ok(())
    }
}