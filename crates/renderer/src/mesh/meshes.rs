use awsm_renderer_core::pipeline::primitive::IndexFormat;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::buffers::dynamic::DynamicBufferKind;
use crate::buffers::dynamic_buddy::DynamicBuddyBuffer;

use super::error::{AwsmMeshError, Result};
use super::morphs::Morphs;
use super::{Mesh, MeshBufferIndexInfo, MeshBufferVertexInfo};

const MESH_INDICES_INITIAL_SIZE: usize = 4096; // 4kB is a good starting point
const MESH_VERTICES_INITIAL_SIZE: usize = 4096; // 4kB is a good starting point

pub struct Meshes {
    list: DenseSlotMap<MeshKey, Mesh>,
    vertex_buffers: DynamicBuddyBuffer<MeshKey>,
    index_buffers: DynamicBuddyBuffer<MeshKey>,
    vertex_infos: SecondaryMap<MeshKey, MeshBufferVertexInfo>,
    index_infos: SecondaryMap<MeshKey, MeshBufferIndexInfo>,
    vertex_dirty: bool,
    index_dirty: bool,
    pub morphs: Morphs,
}

impl Meshes {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            list: DenseSlotMap::with_key(),
            index_buffers: DynamicBuddyBuffer::new(
                MESH_INDICES_INITIAL_SIZE,
                DynamicBufferKind::new_index(),
                gpu,
                Some("MeshIndexBuffer".to_string()),
            )?,
            vertex_buffers: DynamicBuddyBuffer::new(
                MESH_VERTICES_INITIAL_SIZE,
                DynamicBufferKind::new_vertex(),
                gpu,
                Some("MeshVertexBuffer".to_string()),
            )?,
            vertex_infos: SecondaryMap::new(),
            index_infos: SecondaryMap::new(),
            index_dirty: true,
            vertex_dirty: true,
            morphs: Morphs::new(gpu)?,
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
        &self.vertex_buffers.gpu_buffer
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
        &self.index_buffers.gpu_buffer
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

    pub fn write_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        if self.vertex_dirty {
            self.vertex_buffers.write_to_gpu(gpu)?;
            self.vertex_dirty = false;
        }
        if self.index_dirty {
            self.index_buffers.write_to_gpu(gpu)?;
            self.index_dirty = false;
        }
        self.morphs.write_gpu(gpu)?;
        Ok(())
    }
}

new_key_type! {
    pub struct MeshKey;
}
