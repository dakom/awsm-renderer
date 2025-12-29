use awsm_renderer::{
    core::command::{
        render_pass::{ColorAttachment, DepthStencilAttachment, RenderPassDescriptor},
        LoadOp, StoreOp,
    },
    error::AwsmError,
    pipelines::render_pipeline::RenderPipelineKey,
    render::RenderContext,
    AwsmRenderer,
};

pub fn render_grid(
    ctx: &RenderContext,
    grid_bind_group: &web_sys::GpuBindGroup,
    grid_pipeline_key: RenderPipelineKey,
) -> std::result::Result<(), AwsmError> {
    let render_pass = ctx.command_encoder.begin_render_pass(
        &RenderPassDescriptor {
            label: Some("Grid Render Pass"),
            color_attachments: vec![ColorAttachment::new(
                &ctx.render_texture_views.transparent,
                LoadOp::Load,
                StoreOp::Store,
            )],
            depth_stencil_attachment: Some(
                DepthStencilAttachment::new(&ctx.render_texture_views.depth)
                    .with_depth_load_op(LoadOp::Load)
                    .with_depth_store_op(StoreOp::Store),
            ),
            ..Default::default()
        }
        .into(),
    )?;

    render_pass.set_bind_group(0, grid_bind_group, None)?;

    render_pass.set_pipeline(ctx.pipelines.render.get(grid_pipeline_key)?);
    render_pass.draw(3);
    render_pass.end();

    Ok(())
}

pub fn render_gizmos(
    renderer: &mut AwsmRenderer,
    gizmos_pipeline: &web_sys::GpuRenderPipeline,
) -> std::result::Result<(), AwsmError> {
    let command_encoder = renderer.gpu.create_command_encoder(Some("gizmos"));
    let render_pass = command_encoder.begin_render_pass(
        &RenderPassDescriptor {
            label: Some("Gizmos Render Pass"),
            color_attachments: vec![ColorAttachment::new(
                &renderer.gpu.current_context_texture_view()?,
                LoadOp::Load,
                StoreOp::Store,
            )],
            ..Default::default()
        }
        .into(),
    )?;

    render_pass.set_pipeline(gizmos_pipeline);
    render_pass.draw(3);
    render_pass.end();
    renderer.gpu.submit_commands(&command_encoder.finish());
    Ok(())
}
