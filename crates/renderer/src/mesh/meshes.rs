use std::collections::HashMap;

use awsm_renderer_core::buffers::{BufferDescriptor, BufferUsage};
use awsm_renderer_core::pipeline::primitive::IndexFormat;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::Mat4;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::bind_groups::{BindGroupCreate, BindGroups};
use crate::buffer::dynamic_storage::DynamicStorageBuffer;
use crate::buffer::dynamic_uniform::DynamicUniformBuffer;
use crate::materials::Materials;
use crate::mesh::meta::{
    mesh_meta_data, MESH_META_BYTE_ALIGNMENT, MESH_META_BYTE_SIZE, MESH_META_INITIAL_CAPACITY,
};
use crate::mesh::skins::Skins;
use crate::mesh::MeshBufferInfo;
use crate::transforms::TransformKey;
use crate::AwsmRendererLogging;

use super::error::{AwsmMeshError, Result};
use super::morphs::Morphs;
use super::{Mesh, MeshBufferIndexInfo, MeshBufferVertexInfo};

pub struct Meshes {
    list: DenseSlotMap<MeshKey, Mesh>,
    transform_to_meshes: SecondaryMap<TransformKey, Vec<MeshKey>>,
    buffer_infos: SecondaryMap<MeshKey, MeshBufferInfo>,
    // visibility data buffers (position, triangle-id, barycentric)
    visibility_data_buffers: DynamicStorageBuffer<MeshKey>,
    visibility_data_gpu_buffer: web_sys::GpuBuffer,
    visibility_data_dirty: bool,
    // visibility index buffers (position, triangle-id, barycentric)
    visibility_index_buffers: DynamicStorageBuffer<MeshKey>,
    visibility_index_gpu_buffer: web_sys::GpuBuffer,
    visibility_index_dirty: bool,
    // attribute data buffers
    attribute_data_buffers: DynamicStorageBuffer<MeshKey>,
    attribute_data_gpu_buffer: web_sys::GpuBuffer,
    attribute_data_dirty: bool,
    // attribute index buffers (normals, uvs, colors, etc.)
    attribute_index_buffers: DynamicStorageBuffer<MeshKey>,
    attribute_index_gpu_buffer: web_sys::GpuBuffer,
    attribute_index_dirty: bool,
    // meta data buffers
    meta_data_buffers: DynamicUniformBuffer<MeshKey>,
    meta_data_gpu_buffer: web_sys::GpuBuffer,
    meta_data_dirty: bool,
    // morphs and skins
    pub morphs: Morphs,
    pub skins: Skins,
}
impl Meshes {
    // Initial sizes assume ~1000 vertices per mesh
    // but this is just an allocation, can be divided many ways
    const INDICES_INITIAL_SIZE: usize = MESH_META_INITIAL_CAPACITY * 3 * 1000;
    const VERTICES_INITIAL_SIZE: usize = Self::INDICES_INITIAL_SIZE * 24;
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            list: DenseSlotMap::with_key(),
            transform_to_meshes: SecondaryMap::new(),
            buffer_infos: SecondaryMap::new(),
            // visibility data
            visibility_data_buffers: DynamicStorageBuffer::new(
                Self::VERTICES_INITIAL_SIZE,
                Some("MeshVisibilityData".to_string()),
            ),
            visibility_data_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshVisibilityData"),
                    Self::VERTICES_INITIAL_SIZE,
                    BufferUsage::new().with_copy_dst().with_vertex(),
                )
                .into(),
            )?,
            visibility_data_dirty: true,
            // visibility index
            visibility_index_buffers: DynamicStorageBuffer::new(
                Self::INDICES_INITIAL_SIZE,
                Some("MeshVisibilityIndex".to_string()),
            ),
            visibility_index_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshVisibilityIndex"),
                    Self::INDICES_INITIAL_SIZE,
                    BufferUsage::new().with_copy_dst().with_index(),
                )
                .into(),
            )?,
            visibility_index_dirty: true,
            // attribute data
            attribute_data_buffers: DynamicStorageBuffer::new(
                Self::VERTICES_INITIAL_SIZE,
                Some("MeshAttributeData".to_string()),
            ),
            attribute_data_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshAttributeData"),
                    Self::VERTICES_INITIAL_SIZE,
                    BufferUsage::new().with_copy_dst().with_storage(),
                )
                .into(),
            )?,
            attribute_data_dirty: true,
            // attribute index
            attribute_index_buffers: DynamicStorageBuffer::new(
                Self::INDICES_INITIAL_SIZE,
                Some("MeshAttributeIndex".to_string()),
            ),
            attribute_index_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshAttributeIndex"),
                    Self::INDICES_INITIAL_SIZE,
                    BufferUsage::new().with_copy_dst().with_storage(),
                )
                .into(),
            )?,
            attribute_index_dirty: true,
            // meta
            meta_data_buffers: DynamicUniformBuffer::new(
                MESH_META_INITIAL_CAPACITY,
                MESH_META_BYTE_SIZE,
                MESH_META_BYTE_ALIGNMENT,
                Some("MeshMetaData".to_string()),
            ),
            meta_data_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshMetaData"),
                    MESH_META_INITIAL_CAPACITY * MESH_META_BYTE_ALIGNMENT,
                    BufferUsage::new().with_copy_dst().with_uniform(),
                )
                .into(),
            )?,
            meta_data_dirty: true,
            // attribute morphs and skins
            morphs: Morphs::new(gpu)?,
            skins: Skins::new(gpu)?,
        })
    }

    pub fn insert(
        &mut self,
        mesh: Mesh,
        buffer_info: MeshBufferInfo,
        materials: &Materials,
        visibility_data: &[u8],
        // visibility index will be auto-generated
        attribute_data: &[u8],
        attribute_index: &[u8],
    ) -> Result<MeshKey> {
        // TODO - mesh info uniform buffer

        let transform_key = mesh.transform_key;
        let geometry_morph_key = mesh.geometry_morph_key;
        let material_morph_key = mesh.material_morph_key;
        let skin_key = mesh.skin_key;
        let material_key = mesh.material_key;
        let key = self.list.insert(mesh);

        self.transform_to_meshes
            .entry(transform_key)
            .unwrap()
            .or_default()
            .push(key);

        // visibility - index
        let mut visibility_index = Vec::new();
        for i in 0..buffer_info.vertex.count {
            visibility_index.extend_from_slice(&(i as u32).to_le_bytes());
        }
        self.visibility_index_buffers.update(key, &visibility_index);
        self.visibility_index_dirty = true;

        // visibility - data
        self.visibility_data_buffers.update(key, visibility_data);
        self.visibility_data_dirty = true;

        // attributes - index
        self.attribute_index_buffers.update(key, attribute_index);
        self.attribute_index_dirty = true;

        // attributes - data
        self.attribute_data_buffers.update(key, attribute_data);
        self.attribute_data_dirty = true;

        // meta - data
        let meta_data = mesh_meta_data(
            key,
            material_key,
            geometry_morph_key,
            material_morph_key,
            skin_key,
            materials,
            &self.morphs,
            &self.skins,
        )?;
        self.meta_data_buffers.update(key, &meta_data);
        self.meta_data_dirty = true;

        self.buffer_infos.insert(key, buffer_info);

        Ok(key)
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
        self.skins.update_transforms(dirty_transforms);
    }

    pub fn keys(&self) -> impl Iterator<Item = MeshKey> + '_ {
        self.list.keys()
    }

    pub fn visibility_data_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.visibility_data_gpu_buffer
    }
    pub fn visibility_data_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.visibility_data_buffers
            .offset(key)
            .ok_or(AwsmMeshError::MeshNotFound(key))
    }

    pub fn visibility_index_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.visibility_index_gpu_buffer
    }
    pub fn visibility_index_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.visibility_index_buffers
            .offset(key)
            .ok_or(AwsmMeshError::MeshNotFound(key))
    }

    pub fn attribute_data_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.attribute_data_gpu_buffer
    }
    pub fn attribute_data_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.attribute_data_buffers
            .offset(key)
            .ok_or(AwsmMeshError::MeshNotFound(key))
    }

    pub fn attribute_index_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.attribute_index_gpu_buffer
    }
    pub fn attribute_index_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.attribute_index_buffers
            .offset(key)
            .ok_or(AwsmMeshError::MeshNotFound(key))
    }

    pub fn meta_data_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.meta_data_gpu_buffer
    }
    pub fn meta_data_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.meta_data_buffers
            .offset(key)
            .ok_or(AwsmMeshError::MeshNotFound(key))
    }

    pub fn buffer_info(&self, key: MeshKey) -> Result<&MeshBufferInfo> {
        self.buffer_infos
            .get(key)
            .ok_or(AwsmMeshError::MeshNotFound(key))
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
            self.visibility_data_buffers.remove(mesh_key);
            self.visibility_index_buffers.remove(mesh_key);
            self.attribute_data_buffers.remove(mesh_key);
            self.attribute_index_buffers.remove(mesh_key);
            self.meta_data_buffers.remove(mesh_key);

            if self.buffer_infos.remove(mesh_key).is_some() {
                self.visibility_data_dirty = true;
                self.visibility_index_dirty = true;
                self.attribute_data_dirty = true;
                self.attribute_index_dirty = true;
                self.meta_data_dirty = true;
            }

            self.transform_to_meshes
                .get_mut(mesh.transform_key)
                .map(|meshes| meshes.retain(|&key| key != mesh_key));

            let last_transform = if self.transform_to_meshes.contains_key(mesh.transform_key) {
                None
            } else {
                Some(mesh.transform_key)
            };

            if let Some(morph_key) = mesh.geometry_morph_key {
                self.morphs.geometry.remove(morph_key);
            }

            if let Some(morph_key) = mesh.material_morph_key {
                self.morphs.material.remove(morph_key);
            }

            if let Some(skin_key) = mesh.skin_key {
                self.skins.remove(skin_key, last_transform);
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
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        let to_check_dynamic = [
            (
                self.visibility_data_dirty,
                &mut self.visibility_data_buffers,
                &mut self.visibility_data_gpu_buffer,
                BufferUsage::new().with_copy_dst().with_vertex(),
                "MeshVisibilityData",
                None,
            ),
            (
                self.visibility_index_dirty,
                &mut self.visibility_index_buffers,
                &mut self.visibility_index_gpu_buffer,
                BufferUsage::new().with_copy_dst().with_index(),
                "MeshVisibilityIndex",
                None,
            ),
            (
                self.attribute_data_dirty,
                &mut self.attribute_data_buffers,
                &mut self.attribute_data_gpu_buffer,
                BufferUsage::new().with_copy_dst().with_storage(),
                "MeshAttributeData",
                Some(BindGroupCreate::MeshAttributeDataResize),
            ),
            (
                self.attribute_index_dirty,
                &mut self.attribute_index_buffers,
                &mut self.attribute_index_gpu_buffer,
                BufferUsage::new().with_copy_dst().with_storage(),
                "MeshAttributeIndex",
                Some(BindGroupCreate::MeshAttributeIndexResize),
            ),
        ];

        let to_check_uniform = [(
            self.meta_data_dirty,
            &mut self.meta_data_buffers,
            &mut self.meta_data_gpu_buffer,
            BufferUsage::new().with_copy_dst().with_storage(),
            "MeshMetaData",
            Some(BindGroupCreate::MeshMetaResize),
        )];

        let any_dirty = to_check_dynamic.iter().any(|(dirty, _, _, _, _, _)| *dirty)
            || to_check_uniform.iter().any(|(dirty, _, _, _, _, _)| *dirty);

        if any_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Mesh GPU write").entered())
            } else {
                None
            };
            for (dirty, buffer, gpu_buffer, usage, label, bind_group_create) in to_check_dynamic {
                if dirty {
                    if let Some(new_size) = buffer.take_gpu_needs_resize() {
                        *gpu_buffer = gpu.create_buffer(
                            &BufferDescriptor::new(Some(label), new_size, usage).into(),
                        )?;

                        if let Some(create) = bind_group_create {
                            bind_groups.mark_create(create);
                        }
                    }
                    gpu.write_buffer(&gpu_buffer, None, buffer.raw_slice(), None, None)?;
                }
            }

            for (dirty, buffer, gpu_buffer, usage, label, bind_group_create) in to_check_uniform {
                if dirty {
                    if let Some(new_size) = buffer.take_gpu_needs_resize() {
                        *gpu_buffer = gpu.create_buffer(
                            &BufferDescriptor::new(Some(label), new_size, usage).into(),
                        )?;
                        if let Some(create) = bind_group_create {
                            bind_groups.mark_create(create);
                        }
                    }
                    gpu.write_buffer(&gpu_buffer, None, buffer.raw_slice(), None, None)?;
                }
            }

            self.visibility_data_dirty = false;
            self.visibility_index_dirty = false;
            self.attribute_data_dirty = false;
            self.attribute_index_dirty = false;
            self.meta_data_dirty = false;
        }

        Ok(())
    }
}

impl Drop for Meshes {
    fn drop(&mut self) {
        self.visibility_data_gpu_buffer.destroy();
        self.visibility_index_gpu_buffer.destroy();
        self.attribute_data_gpu_buffer.destroy();
        self.attribute_index_gpu_buffer.destroy();
        self.meta_data_gpu_buffer.destroy();
    }
}

new_key_type! {
    pub struct MeshKey;
}
