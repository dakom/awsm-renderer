use std::collections::HashMap;

use awsm_renderer_core::buffers::{BufferDescriptor, BufferUsage};
use awsm_renderer_core::pipeline::primitive::IndexFormat;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::Mat4;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::buffer::dynamic_storage::DynamicStorageBuffer;
use crate::mesh::skins::Skins;
use crate::transforms::TransformKey;
use crate::AwsmRendererLogging;

use super::error::{AwsmMeshError, Result};
use super::morphs::Morphs;
use super::{Mesh, MeshBufferIndexInfo, MeshBufferVertexInfo};

pub struct Meshes {
    list: DenseSlotMap<MeshKey, Mesh>,
    transform_to_meshes: SecondaryMap<TransformKey, Vec<MeshKey>>,
    vertex_buffers: DynamicStorageBuffer<MeshKey>,
    index_buffers: DynamicStorageBuffer<MeshKey>,
    vertex_infos: SecondaryMap<MeshKey, MeshBufferVertexInfo>,
    index_infos: SecondaryMap<MeshKey, MeshBufferIndexInfo>,
    gpu_vertex_buffer: web_sys::GpuBuffer,
    gpu_index_buffer: web_sys::GpuBuffer,
    vertex_dirty: bool,
    index_dirty: bool,
    pub morphs: Morphs,
    pub skins: Skins,
}

impl Meshes {
    const INDICES_INITIAL_SIZE: usize = 4096; // 4kB is a good starting point
    const VERTICES_INITIAL_SIZE: usize = 4096; // 4kB is a good starting point
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            list: DenseSlotMap::with_key(),
            index_buffers: DynamicStorageBuffer::new(
                Self::INDICES_INITIAL_SIZE,
                Some("MeshIndexBuffer".to_string()),
            ),
            vertex_buffers: DynamicStorageBuffer::new(
                Self::VERTICES_INITIAL_SIZE,
                Some("MeshVertexBuffer".to_string()),
            ),
            transform_to_meshes: SecondaryMap::new(),
            gpu_vertex_buffer: gpu_create_vertex_buffer(gpu, Self::VERTICES_INITIAL_SIZE)?,
            gpu_index_buffer: gpu_create_index_buffer(gpu, Self::INDICES_INITIAL_SIZE)?,
            vertex_infos: SecondaryMap::new(),
            index_infos: SecondaryMap::new(),
            index_dirty: true,
            vertex_dirty: true,
            morphs: Morphs::new(gpu)?,
            skins: Skins::new(gpu)?,
        })
    }

    pub fn insert(
        &mut self,
        mesh: Mesh,
        vertex_values: &[u8],
        vertex_info: MeshBufferVertexInfo,
        index: Option<(&[u8], MeshBufferIndexInfo)>,
    ) -> MeshKey {
        let transform_key = mesh.transform_key;
        let key = self.list.insert(mesh);

        self.transform_to_meshes
            .entry(transform_key)
            .unwrap()
            .or_default()
            .push(key);

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

    pub fn update_world(&mut self, dirty_transforms: HashMap<TransformKey, &Mat4>) {
        // This doesn't mark anything as dirty, it just updates the world AABB for frustum culling and depth sorting
        for (transform_key, world_mat) in dirty_transforms.iter() {
            if let Some(mesh_keys) = self.transform_to_meshes.get(*transform_key) {
                for mesh_key in mesh_keys {
                    if let Some(world_aabb) = self
                        .list
                        .get_mut(*mesh_key)
                        .and_then(|m| m.world_aabb.as_mut())
                    {
                        world_aabb.transform(world_mat);
                    }
                }
            }
        }

        // This does update the GPU as dirty, bit skins manage their own GPU dirty state
        self.skins.update_world(dirty_transforms);
    }

    pub fn keys(&self) -> impl Iterator<Item = MeshKey> + '_ {
        self.list.keys()
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

    pub fn get(&self, mesh_key: MeshKey) -> Result<&Mesh> {
        self.list
            .get(mesh_key)
            .ok_or(AwsmMeshError::MeshNotFound(mesh_key))
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

            self.transform_to_meshes
                .get_mut(mesh.transform_key)
                .map(|meshes| meshes.retain(|&key| key != mesh_key));
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
