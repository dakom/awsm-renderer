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
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_groups = MaterialTransparentBindGroups::new(ctx).await?;
        let pipelines = MaterialTransparentPipelines::new(ctx, &bind_groups).await?;

        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    pub async fn update_texture_bindings(
        &mut self,
        ctx: &mut RenderPassInitContext<'_>,
    ) -> Result<()> {
        let bind_groups = MaterialTransparentBindGroups::new(ctx).await?;
        let pipelines = MaterialTransparentPipelines::new(ctx, &bind_groups).await?;

        self.bind_groups = bind_groups;
        self.pipelines = pipelines;
        Ok(())
    }

    pub fn render(&self, ctx: &RenderContext, renderables: Vec<Renderable>) -> Result<()> {
        // TODO!

        Ok(())
    }
}
