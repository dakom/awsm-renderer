use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use crate::{error::Result, render::RenderContext, render_passes::RenderPassInitContext, renderable::{self, Renderable}, AwsmRenderer};

pub struct MaterialTransparentRenderPass {
}

impl MaterialTransparentRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn render(&self, ctx: &RenderContext, renderables: Vec<Renderable>) -> Result<()> {
        // TODO!

        Ok(())
    }
}