use awsm_renderer_core::{
    command::compute_pass::ComputePassDescriptor, renderer::AwsmRendererWebGpu,
};

use crate::{
    error::Result,
    render::RenderContext,
    render_passes::{
        composite::{bind_group::CompositeBindGroups, pipeline::CompositePipelines},
        RenderPassInitContext,
    },
    AwsmRenderer,
};

pub struct CompositeRenderPass {
    pub pipelines: CompositePipelines,
    pub bind_groups: CompositeBindGroups,
}

impl CompositeRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_groups = CompositeBindGroups::new(ctx).await?;
        let pipelines = CompositePipelines::new(ctx, &bind_groups).await?;

        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    pub fn render(&self, ctx: &RenderContext) -> Result<()> {
        let compute_pass = ctx.command_encoder.begin_compute_pass(Some(
            &ComputePassDescriptor::new(Some("Composite Compute Pass")).into(),
        ));

        let bind_group = self.bind_groups.get_bind_group()?;
        let compute_pipeline = if ctx.anti_aliasing.msaa_sample_count.is_some() {
            ctx.pipelines
                .compute
                .get(self.pipelines.multisampled_compute_pipeline_key)?
        } else {
            ctx.pipelines
                .compute
                .get(self.pipelines.singlesampled_compute_pipeline_key)?
        };

        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, None)?;

        let workgroup_size_x = ctx.render_texture_views.width.div_ceil(8);
        let workgroup_size_y = ctx.render_texture_views.height.div_ceil(8);
        compute_pass.dispatch_workgroups(workgroup_size_x, Some(workgroup_size_y), Some(1));
        compute_pass.end();

        Ok(())
    }
}
