use awsm_renderer_core::command::render_pass::{
    ColorAttachment, RenderPassDescriptor, RenderPassEncoder,
};
use awsm_renderer_core::command::{LoadOp, StoreOp};

use crate::bind_groups::BindGroups;
use crate::core::command::CommandEncoder;
use crate::error::Result;
use crate::instances::Instances;
use crate::mesh::Meshes;
use crate::skin::Skins;
use crate::transform::Transforms;
use crate::AwsmRenderer;

impl AwsmRenderer {
    // this should only be called once per frame
    // the various underlying raw data can be updated on their own cadence
    // or just call .update_all() right before .render() for convenience
    pub fn render(&mut self) -> Result<()> {
        let _maybe_span_guard = if self.logging.render_timings {
            Some(tracing::span!(tracing::Level::INFO, "Render").entered())
        } else {
            None
        };

        self.transforms
            .write_gpu(&self.logging, &self.gpu, &mut self.bind_groups)?;
        self.instances.write_gpu(&self.logging, &self.gpu)?;
        self.skins
            .write_gpu(&self.logging, &self.gpu, &mut self.bind_groups)?;
        self.meshes
            .morphs
            .write_gpu(&self.logging, &self.gpu, &mut self.bind_groups)?;
        self.meshes.write_gpu(&self.logging, &self.gpu)?;
        self.camera
            .write_gpu(&self.logging, &self.gpu, &self.bind_groups)?;

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
            meshes: &self.meshes,
            skins: &self.skins,
            instances: &self.instances,
            bind_groups: &self.bind_groups,
        };

        ctx.render_pass.set_bind_group(
            0,
            ctx.bind_groups.buffers.gpu_universal_bind_group(),
            None,
        )?;

        for (key, mesh) in self.meshes.iter() {
            mesh.push_commands(&mut ctx, key)?;
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
    pub meshes: &'a Meshes,
    pub skins: &'a Skins,
    pub instances: &'a Instances,
    pub bind_groups: &'a BindGroups,
}
