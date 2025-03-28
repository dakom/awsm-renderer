pub mod error;

use error::Result;

use crate::AwsmRenderer;

impl AwsmRenderer {
    pub fn render(&self) -> Result<()> {
        // TODO - implement the render function
        // This will include setting up the render pipeline, command encoder, and render pass
        // and submitting the commands to the GPU for rendering
        tracing::info!("Rendering...");
        Ok(())
    }
}
// async fn basic_render(&self) -> Result<()> {
//     static INIT_SHADER_CODE: &str = include_str!("wip-shaders/init.wgsl");
//     let shader = self.gpu.compile_shader(&ShaderModuleDescriptor::new(INIT_SHADER_CODE, None).into());

//     let vertex = VertexState::new(&shader, None);
//     let fragment = FragmentState::new(
//         &shader,
//         None,
//         vec![ColorTargetState::new(self.gpu.current_context_format())],
//     );

//     let pipeline_descriptor = RenderPipelineDescriptor::new(vertex, None).with_fragment(fragment);

//     tracing::info!("Creating pipeline...");

//     let pipeline = self.gpu.create_pipeline(&pipeline_descriptor.into()).await?;

//     tracing::info!("Creating commands...");

//     let command_encoder = self.gpu.create_command_encoder(None);

//     let render_pass = command_encoder.begin_render_pass(
//         &RenderPassDescriptor {
//             color_attachments: vec![ColorAttachment::new(
//                 &self.gpu.current_context_texture_view()?,
//                 LoadOp::Clear,
//                 StoreOp::Store,
//             )],
//             ..Default::default()
//         }
//         .into(),
//     )?;

//     render_pass.set_pipeline(&pipeline);
//     render_pass.draw(3);
//     render_pass.end();

//     tracing::info!("Rendering...");

//     self.gpu.submit_commands(&command_encoder.finish());

//     Ok(())
// }
