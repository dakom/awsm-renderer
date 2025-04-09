use awsm_renderer_core::command::render_pass::{
    ColorAttachment, RenderPassDescriptor, RenderPassEncoder,
};
use awsm_renderer_core::command::{LoadOp, StoreOp};

use crate::core::command::CommandEncoder;
use crate::error::Result;
use crate::shaders::BindGroup;
use crate::transform::Transforms;
use crate::AwsmRenderer;

impl AwsmRenderer {
    pub fn render(&mut self) -> Result<()> {
        self.transforms.write_buffers(&self.gpu)?;
        self.camera.write_buffers(&self.gpu)?;

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
            current_texture_view: &current_texture_view,
            command_encoder,
            render_pass,
            transforms: &self.transforms,
        };

        ctx.render_pass
            .set_bind_group(BindGroup::Camera as u32, &self.camera.bind_group, None)?;

        for mesh in self.meshes.iter() {
            mesh.push_commands(&mut ctx)?;
        }

        ctx.render_pass.end();

        self.gpu.submit_commands(&ctx.command_encoder.finish());

        Ok(())
    }
}

pub struct RenderContext<'a> {
    pub current_texture_view: &'a web_sys::GpuTextureView,
    pub command_encoder: CommandEncoder,
    pub render_pass: RenderPassEncoder,
    pub transforms: &'a Transforms,
}
