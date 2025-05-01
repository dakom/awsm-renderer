use awsm_renderer_core::buffers::{BufferDescriptor, BufferUsage};
use awsm_renderer_core::pipeline::primitive::IndexFormat;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::buffer::dynamic_buddy::DynamicBuddyBuffer;
use crate::AwsmRendererLogging;

use super::error::{AwsmMeshError, Result};
use super::morphs::Morphs;
use super::{Mesh, MeshBufferIndexInfo, MeshBufferVertexInfo};

pub struct Meshes {
    list: DenseSlotMap<MeshKey, Mesh>,
    vertex_buffers: DynamicBuddyBuffer<MeshKey>,
    index_buffers: DynamicBuddyBuffer<MeshKey>,
    vertex_infos: SecondaryMap<MeshKey, MeshBufferVertexInfo>,
    index_infos: SecondaryMap<MeshKey, MeshBufferIndexInfo>,
    gpu_vertex_buffer: web_sys::GpuBuffer,
    gpu_index_buffer: web_sys::GpuBuffer,
    vertex_dirty: bool,
    index_dirty: bool,
    pub morphs: Morphs,
}

impl Meshes {
    const INDICES_INITIAL_SIZE: usize = 4096; // 4kB is a good starting point
    const VERTICES_INITIAL_SIZE: usize = 4096; // 4kB is a good starting point
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            list: DenseSlotMap::with_key(),
            index_buffers: DynamicBuddyBuffer::new(
                Self::INDICES_INITIAL_SIZE,
                Some("MeshIndexBuffer".to_string()),
            ),
            vertex_buffers: DynamicBuddyBuffer::new(
                Self::VERTICES_INITIAL_SIZE,
                Some("MeshVertexBuffer".to_string()),
            ),
            gpu_vertex_buffer: gpu_create_vertex_buffer(gpu, Self::VERTICES_INITIAL_SIZE)?,
            gpu_index_buffer: gpu_create_index_buffer(gpu, Self::INDICES_INITIAL_SIZE)?,
            vertex_infos: SecondaryMap::new(),
            index_infos: SecondaryMap::new(),
            index_dirty: true,
            vertex_dirty: true,
            morphs: Morphs::new(),
        })
    }

    pub fn insert(
        &mut self,
        mesh: Mesh,
        vertex_values: &[u8],
        vertex_info: MeshBufferVertexInfo,
        index: Option<(&[u8], MeshBufferIndexInfo)>,
    ) -> MeshKey {
        let key = self.list.insert(mesh);

        self.vertex_buffers.update(key, vertex_values);
        self.vertex_infos.insert(key, vertex_info);
        self.vertex_dirty = true;

        if let Some((index_values, index_info)) = index {
            self.index_buffers.update(key, index_values);
            self.index_infos.insert(key, index_info);
            self.index_dirty = true;
        }

        key
    }

    pub fn gpu_vertex_buffer(&self) -> &web_sys::GpuBuffer {
        &self.gpu_vertex_buffer
    }

    pub fn vertex_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.vertex_buffers
            .offset(key)
            .ok_or(AwsmMeshError::MeshNotFound(key))
    }

    pub fn index_buffer_offset_format(&self, key: MeshKey) -> Result<(usize, IndexFormat)> {
        let offset = self
            .index_buffers
            .offset(key)
            .ok_or(AwsmMeshError::MeshNotFound(key))?;

        let format = self
            .index_infos
            .get(key)
            .ok_or(AwsmMeshError::MeshNotFound(key))?
            .format;

        Ok((offset, format))
    }

    pub fn gpu_index_buffer(&self) -> &web_sys::GpuBuffer {
        &self.gpu_index_buffer
    }

    pub fn iter(&self) -> impl Iterator<Item = (MeshKey, &Mesh)> {
        self.list.iter()
    }

    pub fn get_mut(&mut self, mesh_key: MeshKey) -> Result<&mut Mesh> {
        self.list
            .get_mut(mesh_key)
            .ok_or(AwsmMeshError::MeshNotFound(mesh_key))
    }

    pub fn remove(&mut self, mesh_key: MeshKey) -> Option<Mesh> {
        if let Some(mesh) = self.list.remove(mesh_key) {
            self.vertex_buffers.remove(mesh_key);
            self.vertex_infos.remove(mesh_key);
            self.vertex_dirty = true;

            self.index_buffers.remove(mesh_key);
            if self.index_infos.remove(mesh_key).is_some() {
                self.index_dirty = true;
            }

            if let Some(morph_key) = mesh.morph_key {
                self.morphs.remove(morph_key);
            }
            Some(mesh)
        } else {
            None
        }
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
    ) -> Result<()> {
        if self.vertex_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Mesh vertex GPU write").entered())
            } else {
                None
            };
            if let Some(new_size) = self.vertex_buffers.take_gpu_needs_resize() {
                self.gpu_vertex_buffer = gpu_create_vertex_buffer(gpu, new_size)?;
            }
            gpu.write_buffer(
                &self.gpu_vertex_buffer,
                None,
                self.vertex_buffers.raw_slice(),
                None,
                None,
            )?;
            self.vertex_dirty = false;
        }
        if self.index_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Mesh index GPU write").entered())
            } else {
                None
            };
            if let Some(new_size) = self.index_buffers.take_gpu_needs_resize() {
                self.gpu_index_buffer = gpu_create_index_buffer(gpu, new_size)?;
            }
            gpu.write_buffer(
                &self.gpu_index_buffer,
                None,
                self.index_buffers.raw_slice(),
                None,
                None,
            )?;
            self.index_dirty = false;
        }
        Ok(())
    }
}

fn gpu_create_vertex_buffer(gpu: &AwsmRendererWebGpu, size: usize) -> Result<web_sys::GpuBuffer> {
    Ok(gpu.create_buffer(
        &BufferDescriptor::new(
            Some("MeshVertex"),
            size,
            BufferUsage::new().with_copy_dst().with_vertex(),
        )
        .into(),
    )?)
}

fn gpu_create_index_buffer(gpu: &AwsmRendererWebGpu, size: usize) -> Result<web_sys::GpuBuffer> {
    Ok(gpu.create_buffer(
        &BufferDescriptor::new(
            Some("MeshIndex"),
            size,
            BufferUsage::new().with_copy_dst().with_index(),
        )
        .into(),
    )?)
}

impl Drop for Meshes {
    fn drop(&mut self) {
        self.gpu_vertex_buffer.destroy();
        self.gpu_index_buffer.destroy();
    }
}

new_key_type! {
    pub struct MeshKey;
}
