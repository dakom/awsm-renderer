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
use awsm_renderer_core::renderer::AwsmRendererWebGpu;

pub struct MaterialTransparentRenderPass {
    pub bind_groups: MaterialTransparentBindGroups,
    pub pipelines: MaterialTransparentPipelines,
}

impl MaterialTransparentRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext) -> Result<Self> {
        let bind_groups = MaterialTransparentBindGroups::new(ctx).await?;
        let pipelines = MaterialTransparentPipelines::new(ctx, &bind_groups).await?;

        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    pub fn render(&self, ctx: &RenderContext, renderables: Vec<Renderable>) -> Result<()> {
        // TODO!

        Ok(())
    }
}
