use crate::error::Result;
use crate::AwsmRenderer;
use crate::core::command::CommandEncoder;

impl AwsmRenderer {
    pub fn render(&self) -> Result<()> {
        // TODO - implement the render function
        // This will include setting up the render pipeline, command encoder, and render pass
        // and submitting the commands to the GPU for rendering
        let mut ctx = RenderContext {
            current_texture_view: self.gpu.current_context_texture_view()?,
            command_encoder: self.gpu.create_command_encoder(Some("Meshes"))
        };

        for (mesh_key, mesh) in self.meshes.iter_with_key() {
            mesh.push_commands(mesh_key, &mut ctx)?;
        }

        self.gpu.submit_commands(&ctx.command_encoder.finish());

        Ok(())
    }
}

pub struct RenderContext {
    pub current_texture_view: web_sys::GpuTextureView,
    pub command_encoder: CommandEncoder
}