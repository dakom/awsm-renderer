use awsm_renderer_core::{
    command::{
        render_pass::{ColorAttachment, RenderPassDescriptor},
        LoadOp, StoreOp,
    },
    error::Result,
    pipeline::{
        fragment::{ColorTargetState, FragmentState},
        vertex::VertexState,
        PipelineDescriptor,
    },
    renderer::AwsmRenderer,
    shaders::ShaderCode,
};

pub trait AwsmRendererWipExt {
    #[allow(async_fn_in_trait)]
    async fn temp_render(&self) -> Result<()>;
}

impl AwsmRendererWipExt for AwsmRenderer {
    async fn temp_render(&self) -> Result<()> {
        static INIT_SHADER_CODE: &str = include_str!("wip-shaders/init.wgsl");
        let shader = self.compile_shader(&ShaderCode::new(INIT_SHADER_CODE, None).into());

        let vertex = VertexState::new(&shader, None);
        let fragment = FragmentState::new(
            &shader,
            None,
            vec![ColorTargetState::new(self.current_context_format())],
        );

        let pipeline_descriptor = PipelineDescriptor::new(vertex, None).with_fragment(fragment);

        tracing::info!("Creating pipeline...");

        let pipeline = self.create_pipeline(&pipeline_descriptor.into()).await?;

        tracing::info!("Creating commands...");

        let command_encoder = self.create_command_encoder(None);

        let render_pass = command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![ColorAttachment::new(
                    &self.current_context_texture_view()?,
                    LoadOp::Clear,
                    StoreOp::Store,
                )],
                ..Default::default()
            }
            .into(),
        )?;

        render_pass.set_pipeline(&pipeline);
        render_pass.draw(3);
        render_pass.end();

        tracing::info!("Rendering...");

        self.submit_commands(&command_encoder.finish());

        Ok(())
    }
}
