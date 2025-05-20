use std::borrow::Cow;

use awsm_renderer_core::command::render_pass::{
    ColorAttachment, DepthStencilAttachment, RenderPassDescriptor, RenderPassEncoder,
};
use awsm_renderer_core::command::{LoadOp, StoreOp};

use crate::bind_groups::BindGroups;
use crate::core::command::CommandEncoder;
use crate::error::{AwsmError, Result};
use crate::instances::Instances;
use crate::materials::Materials;
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
        self.materials
            .write_gpu(&self.logging, &self.gpu, &mut self.bind_groups)?;
        self.lights
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

        let depth_stencil_attachment = match self.depth_texture.as_ref() {
            None => None,
            Some(depth_texture) => {
                let view = depth_texture.create_view().map_err(|e| {
                    AwsmError::DepthTextureCreateView(e.as_string().unwrap_or_default())
                })?;
                Some(
                    DepthStencilAttachment::new(Cow::Owned(view))
                        .with_depth_load_op(LoadOp::Clear)
                        .with_depth_store_op(StoreOp::Store)
                        .with_depth_clear_value(1.0),
                )
            }
        };

        let render_pass = command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![ColorAttachment::new(
                    &current_texture_view,
                    LoadOp::Clear,
                    StoreOp::Store,
                )
                .with_clear_color(self.clear_color.clone())],
                depth_stencil_attachment,
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
            materials: &self.materials,
            skins: &self.skins,
            instances: &self.instances,
            bind_groups: &self.bind_groups,
        };

        ctx.render_pass.set_bind_group(
            0,
            ctx.bind_groups.uniform_storages.gpu_universal_bind_group(),
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
    pub materials: &'a Materials,
    pub skins: &'a Skins,
    pub instances: &'a Instances,
    pub bind_groups: &'a BindGroups,
}
