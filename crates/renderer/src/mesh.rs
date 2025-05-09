mod buffer_info;
mod error;
mod meshes;
pub mod morphs;

use awsm_renderer_core::pipeline::primitive::IndexFormat;

use crate::{materials::MaterialKey, render::RenderContext};
use crate::skin::SkinKey;
use crate::transform::TransformKey;
use crate::bounds::Aabb;

pub use buffer_info::*;
pub use error::AwsmMeshError;
pub use meshes::{MeshKey, Meshes};
pub use morphs::MorphKey;

use super::error::Result;

// this is most like a "primitive" in gltf, not the containing "mesh"
// because for non-gltf naming, "mesh" makes more sense
#[derive(Debug)]
pub struct Mesh {
    pub pipeline: web_sys::GpuRenderPipeline,
    pub draw_count: usize, // indices or vertices
    pub aabb: Option<Aabb>,
    pub transform_key: TransformKey,
    pub material_key: MaterialKey,
    pub morph_key: Option<MorphKey>,
    pub skin_key: Option<SkinKey>,
}

#[derive(Debug, Clone)]
pub struct MeshVertexBuffer {
    pub buffer: web_sys::GpuBuffer,
    pub slot: u32,
    pub offset: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct MeshIndexBuffer {
    pub buffer: web_sys::GpuBuffer,
    pub format: IndexFormat,
    pub offset: u64,
    pub size: u64,
}

impl Mesh {
    pub fn new(
        pipeline: web_sys::GpuRenderPipeline,
        draw_count: usize,
        transform_key: TransformKey,
        material_key: MaterialKey,
    ) -> Self {
        Self {
            pipeline,
            draw_count,
            transform_key,
            material_key,
            aabb: None,
            morph_key: None,
            skin_key: None,
        }
    }

    pub fn with_aabb(mut self, aabb: Aabb) -> Self {
        self.aabb = Some(aabb);
        self
    }

    pub fn with_morph_key(mut self, morph_key: MorphKey) -> Self {
        self.morph_key = Some(morph_key);
        self
    }

    pub fn with_skin_key(mut self, skin_key: SkinKey) -> Self {
        self.skin_key = Some(skin_key);
        self
    }

    pub fn push_commands(&self, ctx: &mut RenderContext, mesh_key: MeshKey) -> Result<()> {
        ctx.render_pass.set_pipeline(&self.pipeline);

        let transform_offset = ctx.transforms.buffer_offset(self.transform_key)? as u32;
        ctx.render_pass.set_bind_group(
            1,
            ctx.bind_groups.buffers.gpu_mesh_all_bind_group(),
            Some(&[transform_offset]),
        )?;

        ctx.render_pass.set_bind_group(
            2,
            ctx.bind_groups
                .materials
                .gpu_material_bind_group(self.material_key)?,
            None,
        )?;

        // if _any_ shapes are used, set the bind group
        // unused shapes will simply be ignored (so 0 offset is fine)
        if self.morph_key.is_some() || self.skin_key.is_some() {
            let (morph_weights_offset, morph_values_offset) = match self.morph_key {
                Some(morph_key) => (
                    ctx.meshes.morphs.weights_buffer_offset(morph_key)? as u32,
                    ctx.meshes.morphs.values_buffer_offset(morph_key)? as u32,
                ),
                None => (0, 0),
            };

            let skin_offset = match self.skin_key {
                Some(skin_key) => ctx.skins.joint_matrices_offset(skin_key)? as u32,
                None => 0,
            };

            ctx.render_pass.set_bind_group(
                3,
                ctx.bind_groups.buffers.gpu_mesh_shape_bind_group(),
                Some(&[morph_weights_offset, morph_values_offset, skin_offset]),
            )?;
        }

        ctx.render_pass.set_vertex_buffer(
            0,
            ctx.meshes.gpu_vertex_buffer(),
            Some(ctx.meshes.vertex_buffer_offset(mesh_key)? as u64),
            None,
        );

        if let Ok(offset) = ctx.instances.transform_buffer_offset(self.transform_key) {
            ctx.render_pass.set_vertex_buffer(
                1,
                ctx.instances.gpu_transform_buffer(),
                Some(offset as u64),
                None,
            );
        }

        let indexed = match ctx.meshes.index_buffer_offset_format(mesh_key).ok() {
            Some((offset, format)) => {
                ctx.render_pass.set_index_buffer(
                    ctx.meshes.gpu_index_buffer(),
                    format,
                    Some(offset as u64),
                    None,
                );
                true
            }
            None => false,
        };

        match (
            indexed,
            ctx.instances.transform_instance_count(self.transform_key),
        ) {
            (false, None) => {
                ctx.render_pass.draw(self.draw_count as u32);
            }
            (true, None) => {
                ctx.render_pass.draw_indexed(self.draw_count as u32);
            }
            (false, Some(instance_count)) => {
                ctx.render_pass
                    .draw_with_instance_count(self.draw_count as u32, instance_count as u32);
            }
            (true, Some(instance_count)) => {
                ctx.render_pass.draw_indexed_with_instance_count(
                    self.draw_count as u32,
                    instance_count as u32,
                );
            }
        }

        Ok(())
    }
}
