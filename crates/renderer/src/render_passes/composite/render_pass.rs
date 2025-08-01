use awsm_renderer_core::renderer::AwsmRendererWebGpu;

use crate::{error::Result, render::RenderContext, render_passes::RenderPassInitContext, AwsmRenderer};

pub struct CompositeRenderPass {
}

impl CompositeRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn render(&self, ctx: &RenderContext) -> Result<()> {
        // TODO!

        Ok(())
    }
}