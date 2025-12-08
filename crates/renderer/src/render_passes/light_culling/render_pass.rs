use crate::{
    error::Result,
    render::RenderContext,
    render_passes::{
        light_culling::{bind_group::LightCullingBindGroups, pipeline::LightCullingPipelines},
        RenderPassInitContext,
    },
};

pub struct LightCullingRenderPass {
    pub bind_groups: LightCullingBindGroups,
    pub pipelines: LightCullingPipelines,
}

impl LightCullingRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_groups = LightCullingBindGroups::new(ctx).await?;
        let pipelines = LightCullingPipelines::new(ctx, &bind_groups).await?;
        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    pub fn render(&self, _ctx: &RenderContext) -> Result<()> {
        // TODO!

        Ok(())
    }
}
