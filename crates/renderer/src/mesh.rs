mod error;
mod meshes;
mod morphs;
mod buffer_info;

use awsm_renderer_core::pipeline::primitive::{IndexFormat, PrimitiveTopology};
use glam::{Mat4, Vec3};

use crate::buffers::bind_group::{BIND_GROUP_TRANSFORM, BIND_GROUP_MORPH_TARGET_VALUES, BIND_GROUP_MORPH_TARGET_WEIGHTS};
use crate::render::RenderContext;
use crate::transform::TransformKey;


pub use error::AwsmMeshError;
pub use meshes::{MeshKey, Meshes};
pub use buffer_info::*;
pub use morphs::MorphKey;

use super::error::Result;

// this is most like a "primitive" in gltf, not the containing "mesh"
// because for non-gltf naming, "mesh" makes more sense
#[derive(Debug)]
pub struct Mesh {
    pub pipeline: web_sys::GpuRenderPipeline,
    pub draw_count: usize, // indices or vertices
    pub vertex_buffers: Vec<MeshVertexBuffer>,
    pub index_buffer: Option<MeshIndexBuffer>,
    pub topology: PrimitiveTopology,
    pub position_extents: Option<PositionExtents>,
    pub transform_key: TransformKey,
    pub morph_key: Option<MorphKey>,
}

#[derive(Debug, Clone)]
pub struct PositionExtents {
    pub min: Vec3,
    pub max: Vec3,
}

impl PositionExtents {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn extend(&mut self, other: &Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    pub fn apply_matrix(&mut self, mat: &Mat4) {
        self.min = mat.transform_point3(self.min);
        self.max = mat.transform_point3(self.max);
    }
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
    ) -> Self {
        Self {
            pipeline,
            draw_count,
            vertex_buffers: Vec::new(),
            index_buffer: None,
            topology: PrimitiveTopology::TriangleList,
            position_extents: None,
            morph_key: None,
            transform_key,
        }
    }

    pub fn with_vertex_buffers(mut self, vertex_buffers: Vec<MeshVertexBuffer>) -> Self {
        self.vertex_buffers = vertex_buffers;
        self
    }

    pub fn with_index_buffer(mut self, index_buffer: MeshIndexBuffer) -> Self {
        self.index_buffer = Some(index_buffer);
        self
    }

    pub fn with_topology(mut self, topology: PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    pub fn with_position_extents(mut self, extents: PositionExtents) -> Self {
        self.position_extents = Some(extents);
        self
    }

    pub fn with_morph_key(mut self, morph_key: MorphKey) -> Self {
        self.morph_key = Some(morph_key);
        self
    }

    pub fn push_commands(&self, ctx: &mut RenderContext) -> Result<()> {
        ctx.render_pass.set_pipeline(&self.pipeline);

        ctx.render_pass.set_bind_group(
            BIND_GROUP_TRANSFORM,
            ctx.transforms.bind_group(),
            Some(&[ctx.transforms.buffer_offset(self.transform_key)? as u32]),
        )?;

        if let Some(morph_key) = self.morph_key {
            ctx.render_pass.set_bind_group(
                BIND_GROUP_MORPH_TARGET_WEIGHTS,
                ctx.meshes.morphs.weights_bind_group(),
                Some(&[
                    ctx.meshes.morphs.weights_buffer_offset(morph_key)? as u32
                ]),
            )?;

            ctx.render_pass.set_bind_group(
                BIND_GROUP_MORPH_TARGET_VALUES,
                ctx.meshes.morphs.values_bind_group(morph_key)?,
                Some(&[
                    ctx.meshes.morphs.values_buffer_offset(morph_key)? as u32
                ]),
            )?;
        }

        for vertex_buffer in &self.vertex_buffers {
            ctx.render_pass.set_vertex_buffer(
                vertex_buffer.slot,
                &vertex_buffer.buffer,
                vertex_buffer.offset,
                vertex_buffer.size,
            );
        }

        match &self.index_buffer {
            Some(index_buffer) => {
                ctx.render_pass.set_index_buffer(
                    &index_buffer.buffer,
                    index_buffer.format,
                    Some(index_buffer.offset),
                    Some(index_buffer.size),
                );
                ctx.render_pass.draw_indexed(self.draw_count as u32);
            }
            None => {
                ctx.render_pass.draw(self.draw_count as u32);
            }
        }

        Ok(())
    }
}
