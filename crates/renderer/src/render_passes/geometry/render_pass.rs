use std::sync::LazyLock;

use awsm_renderer_core::{
    command::{
        color::Color,
        render_pass::{ColorAttachment, DepthStencilAttachment, RenderPassDescriptor},
        LoadOp, StoreOp,
    },
    renderer::AwsmRendererWebGpu,
};

use crate::{
    bind_group_layout::BindGroupLayoutCacheKey,
    error::Result,
    render::RenderContext,
    render_passes::{
        geometry::{bind_group::GeometryBindGroups, pipeline::GeometryPipelines},
        RenderPassInitContext,
    },
    renderable::Renderable,
    AwsmRenderer,
};

static VISIBILITY_CLEAR_COLOR: LazyLock<Color> = LazyLock::new(|| {
    let max = f32::MAX.into();
    Color {
        r: max,
        g: max,
        b: max,
        a: max,
    }
});

pub struct GeometryRenderPass {
    pub bind_groups: GeometryBindGroups,
    pub pipelines: GeometryPipelines,
}

impl GeometryRenderPass {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_groups = GeometryBindGroups::new(ctx).await?;
        let pipelines = GeometryPipelines::new(ctx, &bind_groups).await?;

        Ok(Self {
            bind_groups,
            pipelines,
        })
    }

    pub fn render(&self, ctx: &RenderContext, renderables: &[Renderable]) -> Result<()> {
        let mut color_attachments = vec![
            ColorAttachment::new(
                &ctx.render_texture_views.visibility_data,
                LoadOp::Clear,
                StoreOp::Store,
            )
            .with_clear_color(VISIBILITY_CLEAR_COLOR.clone()),
            ColorAttachment::new(
                &ctx.render_texture_views.barycentric,
                LoadOp::Clear,
                StoreOp::Store,
            ),
            ColorAttachment::new(
                &ctx.render_texture_views.normal_tangent,
                LoadOp::Clear,
                StoreOp::Store,
            ),
            ColorAttachment::new(
                &ctx.render_texture_views.barycentric_derivatives,
                LoadOp::Clear,
                StoreOp::Store,
            ),
        ];

        let render_pass = ctx.command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                label: Some("Geometry Render Pass"),
                color_attachments,
                depth_stencil_attachment: Some(
                    DepthStencilAttachment::new(&ctx.render_texture_views.depth)
                        .with_depth_load_op(LoadOp::Clear)
                        .with_depth_store_op(StoreOp::Store)
                        .with_depth_clear_value(1.0),
                ),
                ..Default::default()
            }
            .into(),
        )?;

        render_pass.set_bind_group(0, self.bind_groups.camera.get_bind_group()?, None)?;

        render_pass.set_bind_group(
            1,
            self.bind_groups.transform_materials.get_bind_group()?,
            None,
        )?;

        render_pass.set_bind_group(3, self.bind_groups.animation.get_bind_group()?, None)?;

        let mut last_render_pipeline_key = None;
        for renderable in renderables {
            let render_pipeline_key = renderable.geometry_render_pipeline_key(&ctx);
            if last_render_pipeline_key != Some(render_pipeline_key) {
                render_pass.set_pipeline(ctx.pipelines.render.get(render_pipeline_key)?);
                last_render_pipeline_key = Some(render_pipeline_key);
            }

            renderable.push_geometry_pass_commands(ctx, &render_pass, &self.bind_groups)?;
        }

        render_pass.end();

        Ok(())
    }
}
