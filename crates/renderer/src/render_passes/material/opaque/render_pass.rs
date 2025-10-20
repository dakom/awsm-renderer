use crate::{
    error::Result,
    render::{self, RenderContext},
    render_passes::{
        material::opaque::{
            bind_group::MaterialOpaqueBindGroups, pipeline::MaterialOpaquePipelines,
        },
        RenderPassInitContext,
    },
    renderable::Renderable,
    AwsmRenderer,
};
use awsm_renderer_core::{
    command::compute_pass::ComputePassDescriptor, renderer::AwsmRendererWebGpu,
};
use slotmap::SecondaryMap;

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

    pub fn render(&self, ctx: &RenderContext, mut renderables: Vec<Renderable>) -> Result<()> {
        let compute_pass = ctx.command_encoder.begin_compute_pass(Some(
            &ComputePassDescriptor::new(Some("Material Opaque Pass")).into(),
        ));

        let (main_bind_group, texture_bind_group, sampler_bind_group) =
            self.bind_groups.get_bind_groups()?;

        compute_pass.set_bind_group(0u32, &main_bind_group, None)?;
        compute_pass.set_bind_group(1u32, &texture_bind_group, None)?;
        compute_pass.set_bind_group(2u32, &sampler_bind_group, None)?;

        let workgroup_size = (
            ctx.render_texture_views.width.div_ceil(8),
            ctx.render_texture_views.height.div_ceil(8),
        );

        let mut seen_pipeline_keys = SecondaryMap::new();
        for renderable in renderables {
            if let Some(compute_pipeline_key) = renderable.material_opaque_compute_pipeline_key() {
                // only need to dispatch once per pipeline, not per renderable
                if !seen_pipeline_keys.contains_key(compute_pipeline_key) {
                    seen_pipeline_keys.insert(compute_pipeline_key, ());

                    compute_pass.set_pipeline(ctx.pipelines.compute.get(compute_pipeline_key)?);
                    compute_pass.dispatch_workgroups(
                        workgroup_size.0,
                        Some(workgroup_size.1),
                        Some(1),
                    );
                }
            }
        }

        compute_pass.end();

        Ok(())
    }
}
