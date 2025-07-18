pub mod textures;
pub mod post_process;

use awsm_renderer_core::command::color::Color;
use awsm_renderer_core::command::render_pass::{
    ColorAttachment, DepthStencilAttachment, RenderPassDescriptor, RenderPassEncoder,
};
use awsm_renderer_core::command::{LoadOp, StoreOp};
use awsm_renderer_core::texture::TextureFormat;

use crate::bind_groups::BindGroups;
use crate::core::command::CommandEncoder;
use crate::error::Result;
use crate::instances::Instances;
use crate::materials::Materials;
use crate::mesh::Meshes;
use crate::pipeline::{Pipelines, RenderPipelineKey};
use crate::renderable::Renderable;
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

        let texture_views = self.render_textures.views(&self.gpu)?;

        let renderables = self.collect_renderables();

        let command_encoder = self.gpu.create_command_encoder(Some("Render pass"));

        let scene_render_texture_view = match self.post_process.settings.enabled {
            false => &self.gpu.current_context_texture_view()?,
            true => &texture_views.scene,
        };
        let scene_render_pass = command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![ColorAttachment::new(
                    scene_render_texture_view,
                    LoadOp::Clear,
                    StoreOp::Store,
                )
                .with_clear_color(self.clear_color.clone())],
                depth_stencil_attachment: Some( 
                    DepthStencilAttachment::new(&texture_views.depth)
                        .with_depth_load_op(LoadOp::Clear)
                        .with_depth_store_op(StoreOp::Store)
                        .with_depth_clear_value(1.0)
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
        };

        ctx.render_pass.set_bind_group(
            0,
            ctx.bind_groups.uniform_storages.gpu_universal_bind_group(),
            None,
        )?;

        let mut last_pipeline_key: Option<RenderPipelineKey> = None;

        for renderable in renderables {
            let render_pipeline_key = renderable.render_pipeline_key();
            if last_pipeline_key != Some(render_pipeline_key) {
                ctx.render_pass
                    .set_pipeline(ctx.pipelines.get_render_pipeline(render_pipeline_key)?);
                last_pipeline_key = Some(render_pipeline_key);
            }

            renderable.push_commands(&mut ctx)?;
        }

        ctx.render_pass.end();

        if self.post_process.settings.enabled {

            // If post-processing is enabled, we need to set up a new render pass

            // finished main render pass, now we can do post-processing
            // let current_texture_view = self.gpu.current_context_texture_view()?;

            // let post_process_pass = ctx.command_encoder.begin_render_pass(
            //     &RenderPassDescriptor {
            //         color_attachments: vec![ColorAttachment::new(
            //             &current_texture_view,
            //             LoadOp::Clear,
            //             StoreOp::Store,
            //         )
            //         .with_clear_color(Color::BLACK)
            //         ],
            //         depth_stencil_attachment: None,
            //         ..Default::default()
            //     }
            //     .into(),
            // )?;
            // ctx.render_pass = post_process_pass;

            // if last_pipeline_key != Some(self.post_process.render_pipeline_key()) {
            //     ctx.render_pass.set_pipeline(
            //         self.pipelines
            //             .get_render_pipeline(self.post_process.render_pipeline_key())?,
            //     );
            //     #[allow(unused_assignments)]
            //     {
            //         last_pipeline_key = Some(self.post_process.render_pipeline_key());
            //     }
            // }

            // self.post_process.push_commands(&mut ctx, &texture_views.scene)?;
            // ctx.render_pass.end();
        }

        self.gpu.submit_commands(&ctx.command_encoder.finish());

        Ok(())
    }

    pub fn scene_target_texture_format(&self) -> TextureFormat {
        match self.post_process.settings.enabled {
            true => self.render_textures.scene_texture_format,
            false => self.gpu.current_context_format()
        }
    }

    pub fn scene_target_depth_texture_format(&self) -> TextureFormat {
        self.render_textures.depth_texture_format
    }

    fn collect_renderables(&self) -> Vec<Renderable<'_>> {
        let mut renderables = Vec::new();
        for (key, mesh) in self.meshes.iter() {
            let has_alpha = self.materials.has_alpha_blend(mesh.material_key).unwrap_or(false);
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
}

pub struct RenderContext<'a> {
    pub command_encoder: CommandEncoder,
    pub render_pass: RenderPassEncoder,
    pub transforms: &'a Transforms,
    pub meshes: &'a Meshes,
    pub pipelines: &'a Pipelines,
    pub materials: &'a Materials,
    pub skins: &'a Skins,
    pub instances: &'a Instances,
    pub bind_groups: &'a BindGroups,
}
