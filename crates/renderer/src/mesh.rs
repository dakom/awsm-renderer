mod buffer_info;
mod error;
mod meshes;
mod morphs;

use awsm_renderer_core::pipeline::primitive::{IndexFormat, PrimitiveTopology};

use crate::bounds::Aabb;
use crate::buffers::bind_group::{
    BIND_GROUP_MORPH_TARGET_VALUES, BIND_GROUP_MORPH_TARGET_WEIGHTS, BIND_GROUP_TRANSFORM,
};
use crate::render::RenderContext;
use crate::skin::SkinKey;
use crate::transform::TransformKey;

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
    pub topology: PrimitiveTopology,
    pub aabb: Option<Aabb>,
    pub transform_key: TransformKey,
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
    ) -> Self {
        Self {
            pipeline,
            draw_count,
            topology: PrimitiveTopology::TriangleList,
            transform_key,
            aabb: None,
            morph_key: None,
            skin_key: None,
        }
    }

    pub fn with_topology(mut self, topology: PrimitiveTopology) -> Self {
        self.topology = topology;
        self
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

        ctx.render_pass.set_bind_group(
            BIND_GROUP_TRANSFORM,
            ctx.transforms.bind_group(),
            Some(&[ctx.transforms.buffer_offset(self.transform_key)? as u32]),
        )?;

        if let Some(morph_key) = self.morph_key {
            ctx.render_pass.set_bind_group(
                BIND_GROUP_MORPH_TARGET_WEIGHTS,
                ctx.meshes.morphs.weights_bind_group(),
                Some(&[ctx.meshes.morphs.weights_buffer_offset(morph_key)? as u32]),
            )?;

            ctx.render_pass.set_bind_group(
                BIND_GROUP_MORPH_TARGET_VALUES,
                ctx.meshes.morphs.values_bind_group(),
                Some(&[ctx.meshes.morphs.values_buffer_offset(morph_key)? as u32]),
            )?;
        }

        ctx.render_pass.set_vertex_buffer(
            0,
            ctx.meshes.gpu_vertex_buffer(),
            Some(ctx.meshes.vertex_buffer_offset(mesh_key)? as u64),
            None,
        );

        match ctx.meshes.index_buffer_offset_format(mesh_key).ok() {
            Some((offset, format)) => {
                ctx.render_pass.set_index_buffer(
                    ctx.meshes.gpu_index_buffer(),
                    format,
                    Some(offset as u64),
                    None,
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
