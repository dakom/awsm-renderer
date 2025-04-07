use awsm_renderer_core::command::render_pass::{
    ColorAttachment, RenderPassDescriptor, RenderPassEncoder,
};
use awsm_renderer_core::command::{LoadOp, StoreOp};

use crate::camera::AwsmCameraError;
use crate::core::command::CommandEncoder;
use crate::error::Result;
use crate::shaders::BindGroup;
use crate::AwsmRenderer;

impl AwsmRenderer {
    pub fn render(&self) -> Result<()> {
        self.pre_render()?;

        let current_texture_view = self.gpu.current_context_texture_view()?;
        let command_encoder = self.gpu.create_command_encoder(Some("Render pass"));

        let render_pass = command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![ColorAttachment::new(
                    &current_texture_view,
                    LoadOp::Clear,
                    StoreOp::Store,
                )],
                ..Default::default()
            }
            .into(),
        )?;

        let mut ctx = RenderContext {
            current_texture_view,
            command_encoder,
            render_pass,
        };

        ctx.render_pass
            .set_bind_group(BindGroup::Camera as u32, &self.camera.bind_group, None)?;

        for (mesh_key, mesh) in self.meshes.iter_with_key() {
            mesh.push_commands(mesh_key, &mut ctx)?;
        }

        ctx.render_pass.end();

        self.gpu.submit_commands(&ctx.command_encoder.finish());

        Ok(())
    }

    fn pre_render(&self) -> Result<()> {
        self.gpu
            .write_buffer(
                &self.camera.gpu_buffer,
                None,
                self.camera.raw_data.as_slice(),
                None,
                None,
            )
            .map_err(AwsmCameraError::WriteBuffer)?;

        // TODO - transforms, etc.

        Ok(())
    }
}

pub struct RenderContext {
    pub current_texture_view: web_sys::GpuTextureView,
    pub command_encoder: CommandEncoder,
    pub render_pass: RenderPassEncoder,
}
