use awsm_renderer_core::command::render_pass::{ColorAttachment, RenderPassDescriptor};
use awsm_renderer_core::command::{LoadOp, StoreOp};
use awsm_renderer_core::pipeline::primitive::{IndexFormat, PrimitiveTopology};
use glam::Vec3;

use crate::error::Result;
use crate::render::RenderContext;

#[derive(Default)]
pub struct Meshes {
    // TODO - replace with slotmap
    lookup: Vec<Mesh>,
}

// TODO - replace with slotmap
pub type MeshKey = usize;

impl Meshes {
    pub fn add(&mut self, mesh: Mesh) -> MeshKey {
        let key = self.lookup.len();
        self.lookup.push(mesh);
        key
    }

    pub fn clear(&mut self) {
        self.lookup.clear();
    }

    pub fn remove(&mut self, key: MeshKey) -> Option<Mesh> {
        if key < self.lookup.len() {
            Some(self.lookup.remove(key))
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Mesh> {
        self.lookup.iter()
    }

    pub fn iter_with_key(&self) -> impl Iterator<Item = (MeshKey, &Mesh)> {
        self.lookup.iter().enumerate()
    }
}

// this is most like a "primitive" in gltf, not the containing "mesh"
// because for non-gltf naming, "mesh" makes more sense
pub struct Mesh {
    pub pipeline: web_sys::GpuRenderPipeline,
    pub draw_count: usize, // indices or vertices
    pub vertex_buffers: Vec<MeshVertexBuffer>,
    pub index_buffer: Option<MeshIndexBuffer>,
    pub topology: PrimitiveTopology,
    pub position_extents: Option<Vec3>,
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
    pub offset: Option<u64>,
    pub size: Option<u64>,
}

impl Mesh {
    pub fn new(pipeline: web_sys::GpuRenderPipeline, draw_count: usize) -> Self {
        Self {
            pipeline,
            draw_count,
            vertex_buffers: Vec::new(),
            index_buffer: None,
            topology: PrimitiveTopology::TriangleList,
            position_extents: None,
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

    pub fn with_position_extents(mut self, position_extents: Vec3) -> Self {
        self.position_extents = Some(position_extents);
        self
    }

    pub fn push_commands(&self, _key: MeshKey, ctx: &mut RenderContext) -> Result<()> {
        let RenderContext {
            current_texture_view,
            command_encoder,
        } = ctx;

        let render_pass = command_encoder.begin_render_pass(
            &RenderPassDescriptor {
                color_attachments: vec![ColorAttachment::new(
                    current_texture_view,
                    LoadOp::Clear,
                    StoreOp::Store,
                )],
                ..Default::default()
            }
            .into(),
        )?;

        render_pass.set_pipeline(&self.pipeline);

        for vertex_buffer in &self.vertex_buffers {
            render_pass.set_vertex_buffer(
                vertex_buffer.slot,
                &vertex_buffer.buffer,
                vertex_buffer.offset,
                vertex_buffer.size,
            );
        }

        match &self.index_buffer {
            Some(index_buffer) => {
                render_pass.set_index_buffer(
                    &index_buffer.buffer,
                    index_buffer.format,
                    index_buffer.offset,
                    index_buffer.size,
                );
                render_pass.draw_indexed(self.draw_count as u32);
            }
            None => {
                render_pass.draw(self.draw_count as u32);
            }
        }

        render_pass.end();

        Ok(())
    }
}
