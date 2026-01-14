use awsm_renderer_core::command::compute_pass::ComputePassDescriptor;

use crate::{
    error::Result,
    render::RenderContext,
    render_passes::{
        effects::{bind_group::EffectsBindGroups, pipeline::EffectsPipelines},
        RenderPassInitContext,
    },
};

pub struct EffectsRenderPass {
    pub bind_groups: EffectsBindGroups,
    pub pipelines: EffectsPipelines,
}

impl EffectsRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_groups = EffectsBindGroups::new(ctx).await?;
        let pipelines = EffectsPipelines::new(ctx, &bind_groups).await?;

        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    pub fn render(&self, ctx: &RenderContext) -> Result<()> {
        let compute_pass = ctx.command_encoder.begin_compute_pass(Some(
            &ComputePassDescriptor::new(Some("Effects Pass")).into(),
        ));

        compute_pass.set_bind_group(0, self.bind_groups.get_bind_group()?, None)?;

        let workgroup_size = (
            ctx.render_texture_views.width.div_ceil(8),
            ctx.render_texture_views.height.div_ceil(8),
        );

        if let Some(compute_pipeline_key) = self.pipelines.compute_pipeline_key {
            compute_pass.set_pipeline(ctx.pipelines.compute.get(compute_pipeline_key)?);
            compute_pass.dispatch_workgroups(workgroup_size.0, Some(workgroup_size.1), Some(1));
        }

        compute_pass.end();

        Ok(())
    }
}
