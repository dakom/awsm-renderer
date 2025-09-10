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

        renderables.sort_by(|a, b| {
            a.material_opaque_compute_pipeline_key()
                .cmp(&b.material_opaque_compute_pipeline_key())
        });

        // these bind groups stay the same, it's only meta that changes
        compute_pass.set_bind_group(0, self.bind_groups.core.get_bind_group()?, None)?;
        let bind_groups = self.bind_groups.textures.get_bind_groups()?;
        for (index, bind_group) in bind_groups.iter().enumerate() {
            compute_pass.set_bind_group(index as u32, &bind_group, None)?;
        }

        let workgroup_size = (
            ctx.render_texture_views.width.div_ceil(8),
            ctx.render_texture_views.height.div_ceil(8),
        );

        let mut last_compute_pipeline_key = None;
        for renderable in renderables {
            if let Some(compute_pipeline_key) = renderable.material_opaque_compute_pipeline_key() {
                if last_compute_pipeline_key != Some(compute_pipeline_key) {
                    compute_pass.set_pipeline(ctx.pipelines.compute.get(compute_pipeline_key)?);
                    last_compute_pipeline_key = Some(compute_pipeline_key);
                }

                renderable.push_material_opaque_pass_commands(
                    ctx,
                    &compute_pass,
                    &self.bind_groups,
                    workgroup_size,
                )?;
            }
        }

        compute_pass.end();

        Ok(())
    }
}
