use awsm_renderer_core::command::color::Color;
use awsm_renderer_core::command::render_pass::{
    ColorAttachment, DepthStencilAttachment, RenderPassDescriptor,
};
use awsm_renderer_core::command::{CommandEncoder, LoadOp, StoreOp};
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use awsm_renderer_core::texture::TextureFormat;

use crate::anti_alias::AntiAliasing;
use crate::bind_groups::{BindGroupCreate, BindGroupRecreateContext, BindGroups};
use crate::error::Result;
use crate::instances::Instances;
use crate::materials::Materials;
use crate::mesh::skins::Skins;
use crate::mesh::Meshes;
use crate::pipelines::Pipelines;
use crate::render_passes::RenderPasses;
use crate::render_textures::{RenderTextureFormats, RenderTextureViews};
use crate::renderable::Renderable;
use crate::transforms::Transforms;
use crate::{AwsmRenderer, AwsmRendererLogging};

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
        self.meshes
            .skins
            .write_gpu(&self.logging, &self.gpu, &mut self.bind_groups)?;
        self.meshes
            .morphs
            .write_gpu(&self.logging, &self.gpu, &mut self.bind_groups)?;
        self.meshes
            .meta
            .write_gpu(&self.logging, &self.gpu, &mut self.bind_groups)?;
        self.textures.write_texture_transforms_gpu(
            &self.logging,
            &self.gpu,
            &mut self.bind_groups,
        )?;
        self.meshes
            .write_gpu(&self.logging, &self.gpu, &mut self.bind_groups)?;
        self.camera
            .write_gpu(&self.logging, &self.gpu, &self.bind_groups)?;

        let render_texture_views = self
            .render_textures
            .views(&self.gpu, self.anti_aliasing.clone())?;

        if render_texture_views.size_changed {
            self.bind_groups
                .mark_create(BindGroupCreate::TextureViewResize);
        }

        self.bind_groups.recreate(
            BindGroupRecreateContext {
                gpu: &self.gpu,
                render_texture_views: &render_texture_views,
                textures: &self.textures,
                materials: &self.materials,
                bind_group_layouts: &mut self.bind_group_layouts,
                meshes: &self.meshes,
                camera: &self.camera,
                environment: &self.environment,
                lights: &self.lights,
                transforms: &self.transforms,
                anti_aliasing: &self.anti_aliasing,
            },
            &mut self.render_passes,
        )?;

        let ctx = RenderContext {
            gpu: &self.gpu,
            command_encoder: self.gpu.create_command_encoder(Some("Rendering")),
            render_texture_views,
            transforms: &self.transforms,
            meshes: &self.meshes,
            materials: &self.materials,
            pipelines: &self.pipelines,
            instances: &self.instances,
            bind_groups: &self.bind_groups,
            render_passes: &self.render_passes,
            anti_aliasing: &self.anti_aliasing,
        };

        let renderables = self.collect_renderables(&ctx)?;

        {
            let _maybe_span_guard = if self.logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Geometry RenderPass").entered())
            } else {
                None
            };

            self.render_passes
                .geometry
                .render(&ctx, &renderables.opaque)?;
        }

        {
            let _maybe_span_guard = if self.logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Light Culling RenderPass").entered())
            } else {
                None
            };

            self.render_passes.light_culling.render(&ctx)?;
        }

        {
            let _maybe_span_guard = if self.logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Clear opaque color").entered())
            } else {
                None
            };

            self.render_textures.clear_opaque_color(&self.gpu)?;
        }

        {
            let _maybe_span_guard = if self.logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Material Opaque RenderPass").entered())
            } else {
                None
            };

            self.render_passes
                .material_opaque
                .render(&ctx, renderables.opaque)?;
        }

        {
            let _maybe_span_guard = if self.logging.render_timings {
                Some(
                    tracing::span!(tracing::Level::INFO, "Material Transparent RenderPass")
                        .entered(),
                )
            } else {
                None
            };

            self.render_passes
                .material_transparent
                .render(&ctx, renderables.transparent)?;
        }

        {
            let _maybe_span_guard = if self.logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Composite RenderPass").entered())
            } else {
                None
            };

            self.render_passes.composite.render(&ctx)?;
        }

        {
            let _maybe_span_guard = if self.logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Display RenderPass").entered())
            } else {
                None
            };

            self.render_passes.display.render(&ctx)?;
        }

        self.gpu.submit_commands(&ctx.command_encoder.finish());

        Ok(())
    }
}

pub struct RenderContext<'a> {
    pub gpu: &'a AwsmRendererWebGpu,
    pub command_encoder: CommandEncoder,
    pub render_texture_views: RenderTextureViews,
    pub transforms: &'a Transforms,
    pub meshes: &'a Meshes,
    pub pipelines: &'a Pipelines,
    pub materials: &'a Materials,
    pub instances: &'a Instances,
    pub bind_groups: &'a BindGroups,
    pub render_passes: &'a RenderPasses,
    pub anti_aliasing: &'a AntiAliasing,
}
