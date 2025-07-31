use awsm_renderer_core::command::color::Color;
use awsm_renderer_core::command::render_pass::{
    ColorAttachment, DepthStencilAttachment, RenderPassDescriptor,
};
use awsm_renderer_core::command::{CommandEncoder, LoadOp, StoreOp};
use awsm_renderer_core::texture::TextureFormat;

use crate::bind_groups::BindGroups;
use crate::error::Result;
use crate::instances::Instances;
use crate::materials::Materials;
use crate::mesh::skins::Skins;
use crate::mesh::Meshes;
use crate::pipelines::Pipelines;
use crate::render_textures::{RenderTextureFormats, RenderTextureViews};
use crate::renderable::Renderable;
use crate::transforms::Transforms;
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

        self.render_textures.next_frame();

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

        self.recreate_marked_bind_groups()?;

        let texture_views = self.render_textures.views(&self.gpu)?;

        let ctx = RenderContext {
            command_encoder: self.gpu.create_command_encoder(Some("Rendering")),
            texture_views,
            transforms: &self.transforms,
            meshes: &self.meshes,
            materials: &self.materials,
            pipelines: &self.pipelines,
            skins: &self.skins,
            instances: &self.instances,
            bind_groups: &self.bind_groups,
        };

        self.render_geometry_pass(&ctx)?;
        self.render_light_culling_pass(&ctx)?;
        self.render_material_opaque_pass(&ctx)?;
        self.render_material_transparent_pass(&ctx)?;
        self.render_composite_pass(&ctx)?;
        self.render_display_pass(&ctx)?;

        self.gpu.submit_commands(&ctx.command_encoder.finish());


        Ok(())
    }
}

pub struct RenderContext<'a> {
    pub command_encoder: CommandEncoder,
    pub texture_views: RenderTextureViews,
    pub transforms: &'a Transforms,
    pub meshes: &'a Meshes,
    pub pipelines: &'a Pipelines,
    pub materials: &'a Materials,
    pub skins: &'a Skins,
    pub instances: &'a Instances,
    pub bind_groups: &'a BindGroups,
}