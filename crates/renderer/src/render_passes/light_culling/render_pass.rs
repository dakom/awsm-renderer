//! Light culling render pass execution.

use crate::{
    error::Result,
    render::RenderContext,
    render_passes::{
        light_culling::{bind_group::LightCullingBindGroups, pipeline::LightCullingPipelines},
        RenderPassInitContext,
    },
};

/// Light culling pass bind groups and pipelines.
pub struct LightCullingRenderPass {
    pub bind_groups: LightCullingBindGroups,
    pub pipelines: LightCullingPipelines,
}

impl LightCullingRenderPass {
    /// Creates the light culling render pass resources.
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_groups = LightCullingBindGroups::new(ctx).await?;
        let pipelines = LightCullingPipelines::new(ctx, &bind_groups).await?;
        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    /// Executes the light culling pass.
    pub fn render(&self, _ctx: &RenderContext) -> Result<()> {
        // TODO!

        Ok(())
    }
}
