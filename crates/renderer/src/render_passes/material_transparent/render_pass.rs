use crate::{
    error::Result,
    render::RenderContext,
    render_passes::{
        material_transparent::{
            bind_group::MaterialTransparentBindGroups, pipeline::MaterialTransparentPipelines,
        },
        RenderPassInitContext,
    },
    renderable::Renderable,
};
use awsm_renderer_core::command::{
    render_pass::{ColorAttachment, DepthStencilAttachment, RenderPassDescriptor},
    LoadOp, StoreOp,
};

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

    pub async fn texture_pool_changed(
        &mut self,
        ctx: &mut RenderPassInitContext<'_>,
    ) -> Result<()> {
        self.bind_groups = self.bind_groups.clone_because_texture_pool_changed(ctx)?;
        self.pipelines = MaterialTransparentPipelines::new(ctx, &self.bind_groups).await?;

        Ok(())
    }

    pub fn render(&self, ctx: &RenderContext, renderables: Vec<Renderable>) -> Result<()> {
        let mut color_attachment = ColorAttachment::new(
            &ctx.render_texture_views.transparent,
            LoadOp::Load,
            StoreOp::Store,
        );

        if ctx.anti_aliasing.msaa_sample_count.is_some() {
            color_attachment =
                color_attachment.with_resolve_target(&ctx.render_texture_views.composite);
        }

        let render_pass = ctx.command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                label: Some("Material Transparent Pass"),
                color_attachments: vec![color_attachment],
                depth_stencil_attachment: Some(
                    DepthStencilAttachment::new(&ctx.render_texture_views.depth)
                        .with_depth_load_op(LoadOp::Load)
                        .with_depth_store_op(StoreOp::Store),
                ),
                ..Default::default()
            }
            .into(),
        )?;

        let (main_bind_group, mesh_material_bind_group, lights_bind_group, texture_bind_group) =
            self.bind_groups.get_bind_groups()?;

        // set later with dynamic offsets
        render_pass.set_bind_group(0u32, main_bind_group, None)?;
        render_pass.set_bind_group(1u32, lights_bind_group, None)?;
        render_pass.set_bind_group(2u32, texture_bind_group, None)?;

        let mut last_render_pipeline_key = None;
        for renderable in renderables {
            if let Some(render_pipeline_key) =
                renderable.material_transparent_render_pipeline_key(ctx)
            {
                if last_render_pipeline_key != Some(render_pipeline_key) {
                    render_pass.set_pipeline(ctx.pipelines.render.get(render_pipeline_key)?);
                    last_render_pipeline_key = Some(render_pipeline_key);
                }

                renderable.push_material_transparent_pass_commands(
                    ctx,
                    &render_pass,
                    mesh_material_bind_group,
                )?;
            }
        }

        render_pass.end();

        Ok(())
    }
}
