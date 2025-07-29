pub mod context;
pub mod post_process;
pub mod textures;

use awsm_renderer_core::command::color::Color;
use awsm_renderer_core::command::render_pass::{
    ColorAttachment, DepthStencilAttachment, RenderPassDescriptor,
};
use awsm_renderer_core::command::{LoadOp, StoreOp};
use awsm_renderer_core::texture::TextureFormat;

use crate::error::Result;
use crate::render::context::RenderContext;
use crate::render::textures::RenderTextureFormats;
use crate::renderable::Renderable;
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

        // this should probably be called first so we get the uniform in this frame
        self.render_textures.next_frame();

        self.post_process
            .uniforms
            .update(self.render_textures.frame_count(), self.camera.moved())?;
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
        self.post_process
            .uniforms
            .write_gpu(&self.logging, &self.gpu, &self.bind_groups)?;

        let (texture_views, views_changed) = self.render_textures.views(&self.gpu)?;

        let ctx = if !self.post_process.settings.enabled {
            self.render_renderables(
                &self.gpu.current_context_texture_view()?,
                // just always render to 0 since we don't have ping-pong
                &texture_views.clip_positions[0],
                &texture_views.depths[0],
                self._clear_color.clone(),
            )?
        } else {
            if views_changed {
                self.post_process_update_view()?;
            }
            let mut ctx = self.render_renderables(
                &texture_views.scene,
                texture_views.clip_position_render_target(),
                texture_views.depth_render_target(),
                self._clear_color_perceptual_to_linear.clone(),
            )?;

            self.render_post_process(
                &mut ctx,
                texture_views.accumulation_render_target(),
                self.render_textures.ping_pong(),
            )?;

            ctx
        };

        self.gpu.submit_commands(&ctx.command_encoder.finish());

        Ok(())
    }

    pub fn renderable_texture_formats(&self) -> RenderTextureFormats {
        let mut texture_formats = self.render_textures.formats.clone();
        if !self.post_process.settings.enabled {
            texture_formats.scene = self.gpu.current_context_format();
        };

        texture_formats
    }

    pub fn scene_target_depth_texture_format(&self) -> TextureFormat {
        self.render_textures.formats.depth
    }

    fn collect_renderables(&self) -> Vec<Renderable<'_>> {
        let mut renderables = Vec::new();
        for (key, mesh) in self.meshes.iter() {
            let has_alpha = self
                .materials
                .has_alpha_blend(mesh.material_key)
                .unwrap_or(false);
            renderables.push(Renderable::Mesh {
                key,
                mesh,
                has_alpha,
            });
        }

        renderables.sort_by(|a, b| {
            // Criterion 1 & 2: Group by has_alpha. Non-alpha (false) comes before alpha (true).
            let alpha_ordering = a.has_alpha().cmp(&b.has_alpha());
            if alpha_ordering != std::cmp::Ordering::Equal {
                return alpha_ordering;
            }

            // Criterion 3: Within alpha groups, group by render_pipeline_key.
            let pipeline_ordering = a.render_pipeline_key().cmp(&b.render_pipeline_key());
            if pipeline_ordering != std::cmp::Ordering::Equal {
                return pipeline_ordering;
            }

            // Criterion 4: Within alpha->pipeline_key groups, sort by depth.
            match (a.transform_key(), b.transform_key()) {
                (Some(a_key), Some(b_key)) => {
                    let a_world_mat = self.transforms.get_world(a_key).unwrap();
                    let b_world_mat = self.transforms.get_world(b_key).unwrap();

                    // w_axis is the translation vector in the world matrix
                    // We use the z component for depth sorting.
                    let a_depth = a_world_mat.w_axis.z;
                    let b_depth = b_world_mat.w_axis.z;

                    if a.has_alpha() {
                        // Sort back-to-front for transparent objects.
                        b_depth
                            .partial_cmp(&a_depth)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        // Sort front-to-back for opaque objects.
                        a_depth
                            .partial_cmp(&b_depth)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }
                }
                _ => std::cmp::Ordering::Equal,
            }
        });

        renderables
    }

    fn render_renderables(
        &self,
        scene_texture_view: &web_sys::GpuTextureView,
        clip_position_texture_view: &web_sys::GpuTextureView,
        depth_texture_view: &web_sys::GpuTextureView,
        clear_color: Color,
    ) -> Result<RenderContext> {
        let renderables = self.collect_renderables();

        let command_encoder = self.gpu.create_command_encoder(Some("Rendering"));
        let scene_render_pass = command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![
                    ColorAttachment::new(scene_texture_view, LoadOp::Clear, StoreOp::Store)
                        .with_clear_color(clear_color),
                    ColorAttachment::new(clip_position_texture_view, LoadOp::Clear, StoreOp::Store),
                ],
                depth_stencil_attachment: Some(
                    DepthStencilAttachment::new(depth_texture_view)
                        .with_depth_load_op(LoadOp::Clear)
                        .with_depth_store_op(StoreOp::Store)
                        .with_depth_clear_value(1.0),
                ),
                ..Default::default()
            }
            .into(),
        )?;

        let mut ctx = RenderContext {
            command_encoder,
            render_pass: scene_render_pass,
            transforms: &self.transforms,
            meshes: &self.meshes,
            materials: &self.materials,
            pipelines: &self.pipelines,
            skins: &self.skins,
            instances: &self.instances,
            bind_groups: &self.bind_groups,
            last_render_pipeline_key: None,
        };

        ctx.render_pass.set_bind_group(
            0,
            ctx.bind_groups.uniform_storages.gpu_universal_bind_group(),
            None,
        )?;

        for renderable in renderables {
            let render_pipeline_key = renderable.render_pipeline_key();
            if ctx.last_render_pipeline_key != Some(render_pipeline_key) {
                ctx.render_pass
                    .set_pipeline(ctx.pipelines.get_render_pipeline(render_pipeline_key)?);
                ctx.last_render_pipeline_key = Some(render_pipeline_key);
            }

            renderable.push_commands(&mut ctx)?;
        }

        ctx.render_pass.end();

        Ok(ctx)
    }

    fn render_post_process(
        &self,
        ctx: &mut RenderContext,
        accumulation_texture_view: &web_sys::GpuTextureView,
        ping_pong: bool,
    ) -> Result<()> {
        let current_texture_view = self.gpu.current_context_texture_view()?;

        let post_process_pass = ctx.command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![
                    ColorAttachment::new(&current_texture_view, LoadOp::Clear, StoreOp::Store),
                    ColorAttachment::new(accumulation_texture_view, LoadOp::Load, StoreOp::Store),
                ],
                depth_stencil_attachment: None,
                ..Default::default()
            }
            .into(),
        )?;
        ctx.render_pass = post_process_pass;

        ctx.render_pass.set_bind_group(
            1,
            ctx.bind_groups
                .uniform_storages
                .gpu_post_process_bind_group(),
            None,
        )?;

        if ctx.last_render_pipeline_key != Some(self.post_process.render_pipeline_key()) {
            ctx.render_pass.set_pipeline(
                self.pipelines
                    .get_render_pipeline(self.post_process.render_pipeline_key())?,
            );
            ctx.last_render_pipeline_key = Some(self.post_process.render_pipeline_key());
        }

        self.post_process.push_commands(ctx, ping_pong)?;
        ctx.render_pass.end();

        Ok(())
    }
}
