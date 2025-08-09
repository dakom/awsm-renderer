use crate::{
    error::Result,
    render::RenderContext,
    render_passes::{
        material::opaque::{
            bind_group::MaterialOpaqueBindGroups, pipeline::MaterialOpaquePipelines,
        },
        RenderPassInitContext,
    },
    AwsmRenderer,
};
use awsm_renderer_core::{
    command::compute_pass::ComputePassDescriptor, renderer::AwsmRendererWebGpu,
};

pub struct MaterialOpaqueRenderPass {
    pub bind_groups: MaterialOpaqueBindGroups,
    pub pipelines: MaterialOpaquePipelines,
}

impl MaterialOpaqueRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_groups = MaterialOpaqueBindGroups::new(ctx).await?;
        let pipelines = MaterialOpaquePipelines::new(ctx, &bind_groups).await?;

        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    pub async fn update_texture_bindings(
        &mut self,
        ctx: &mut RenderPassInitContext<'_>,
    ) -> Result<()> {
        let bind_groups = MaterialOpaqueBindGroups::new(ctx).await?;
        let pipelines = MaterialOpaquePipelines::new(ctx, &bind_groups).await?;

        self.bind_groups = bind_groups;
        self.pipelines = pipelines;

        Ok(())
    }

    pub fn render(&self, ctx: &RenderContext) -> Result<()> {
        let compute_pass = ctx.command_encoder.begin_compute_pass(Some(
            &ComputePassDescriptor::new(Some("Material Opaque Pass")).into(),
        ));

        let bind_groups = self.bind_groups.get_bind_groups()?;
        let compute_pipeline = ctx
            .pipelines
            .compute
            .get(self.pipelines.compute_pipeline_key)?;

        compute_pass.set_pipeline(&compute_pipeline);
        for (index, bind_group) in bind_groups.iter().enumerate() {
            compute_pass.set_bind_group(index as u32, &bind_group, None)?;
        }

        let workgroup_size_x = ctx.render_texture_views.width.div_ceil(8);
        let workgroup_size_y = ctx.render_texture_views.height.div_ceil(8);
        compute_pass.dispatch_workgroups(workgroup_size_x, Some(workgroup_size_y), Some(1));
        compute_pass.end();

        Ok(())
    }
}
