use std::collections::HashMap;

use awsm_renderer_core::buffers::{BufferDescriptor, BufferUsage};
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::Mat4;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::bind_groups::{BindGroupCreate, BindGroups};
use crate::buffer::dynamic_storage::DynamicStorageBuffer;
use crate::materials::Materials;
use crate::mesh::meta::{MeshMeta, MESH_META_INITIAL_CAPACITY};
use crate::mesh::skins::Skins;
use crate::mesh::MeshBufferInfos;
use crate::transforms::{TransformKey, Transforms};
use crate::AwsmRendererLogging;

use super::error::{AwsmMeshError, Result};
use super::morphs::Morphs;
use super::{Mesh, MeshBufferVertexInfo};

pub struct Meshes {
    list: DenseSlotMap<MeshKey, Mesh>,
    transform_to_meshes: SecondaryMap<TransformKey, Vec<MeshKey>>,
    // visibility geometry data buffers (position, triangle-id, barycentric)
    visibility_geometry_data_buffers: DynamicStorageBuffer<MeshKey>,
    visibility_geometry_data_gpu_buffer: web_sys::GpuBuffer,
    visibility_geometry_data_dirty: bool,
    // visibility geometry index buffers (position, triangle-id, barycentric, etc.)
    visibility_geometry_index_buffers: DynamicStorageBuffer<MeshKey>,
    visibility_geometry_index_gpu_buffer: web_sys::GpuBuffer,
    visibility_geometry_index_dirty: bool,
    // transparency geometry data buffers (position, etc.)
    transparency_geometry_data_buffers: DynamicStorageBuffer<MeshKey>,
    transparency_geometry_data_gpu_buffer: web_sys::GpuBuffer,
    transparency_geometry_data_dirty: bool,
    // attribute data buffers
    custom_attribute_data_buffers: DynamicStorageBuffer<MeshKey>,
    custom_attribute_data_gpu_buffer: web_sys::GpuBuffer,
    custom_attribute_data_dirty: bool,
    // attribute index buffers (normals, uvs, colors, etc.)
    custom_attribute_index_buffers: DynamicStorageBuffer<MeshKey>,
    custom_attribute_index_gpu_buffer: web_sys::GpuBuffer,
    custom_attribute_index_dirty: bool,
    // buffer infos
    pub buffer_infos: MeshBufferInfos,
    // meta
    pub meta: MeshMeta,
    // morphs and skins
    pub morphs: Morphs,
    pub skins: Skins,
}
impl Meshes {
    // Initial sizes assume ~1000 vertices per mesh
    // but this is just an allocation, can be divided many ways
    const INDICES_INITIAL_SIZE: usize = MESH_META_INITIAL_CAPACITY * 3 * 1000;

    const VISIBILITY_GEOMETRY_INITIAL_SIZE: usize =
        Self::INDICES_INITIAL_SIZE * MeshBufferVertexInfo::VISIBILITY_GEOMETRY_BYTE_SIZE;

    const TRANSPARENCY_GEOMETRY_INITIAL_SIZE: usize =
        Self::INDICES_INITIAL_SIZE * MeshBufferVertexInfo::TRANSPARENCY_GEOMETRY_BYTE_SIZE;

    // Attribute data is much smaller - only custom attributes (UVs, colors, joints, weights).
    // Estimate: 2 UV sets (8 bytes each) = 16 bytes per vertex as a reasonable starting point.
    // For textureless models this will be 0, but buffer will grow as needed.
    const ATTRIBUTE_DATA_INITIAL_SIZE: usize = Self::INDICES_INITIAL_SIZE * 16;

    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            list: DenseSlotMap::with_key(),
            transform_to_meshes: SecondaryMap::new(),
            buffer_infos: MeshBufferInfos::new(),
            // visibility data
            visibility_geometry_data_buffers: DynamicStorageBuffer::new(
                Self::VISIBILITY_GEOMETRY_INITIAL_SIZE,
                Some("MeshVisibilityData".to_string()),
            ),
            visibility_geometry_data_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshVisibilityData"),
                    Self::VISIBILITY_GEOMETRY_INITIAL_SIZE,
                    BufferUsage::new().with_copy_dst().with_vertex(),
                )
                .into(),
            )?,
            visibility_geometry_data_dirty: true,
            // visibility index
            visibility_geometry_index_buffers: DynamicStorageBuffer::new(
                Self::INDICES_INITIAL_SIZE,
                Some("MeshVisibilityIndex".to_string()),
            ),
            visibility_geometry_index_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshVisibilityIndex"),
                    Self::INDICES_INITIAL_SIZE,
                    BufferUsage::new().with_copy_dst().with_index(),
                )
                .into(),
            )?,
            visibility_geometry_index_dirty: true,
            // transparency geometry
            transparency_geometry_data_buffers: DynamicStorageBuffer::new(
                Self::TRANSPARENCY_GEOMETRY_INITIAL_SIZE,
                Some("MeshTransparencyData".to_string()),
            ),
            transparency_geometry_data_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshTransparencyData"),
                    Self::TRANSPARENCY_GEOMETRY_INITIAL_SIZE,
                    BufferUsage::new().with_copy_dst().with_vertex(),
                )
                .into(),
            )?,
            transparency_geometry_data_dirty: true,
            // attribute data
            custom_attribute_data_buffers: DynamicStorageBuffer::new(
                Self::ATTRIBUTE_DATA_INITIAL_SIZE,
                Some("MeshAttributeData".to_string()),
            ),
            custom_attribute_data_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshAttributeData"),
                    Self::ATTRIBUTE_DATA_INITIAL_SIZE,
                    BufferUsage::new()
                        .with_copy_dst()
                        .with_storage()
                        .with_vertex(),
                )
                .into(),
            )?,
            custom_attribute_data_dirty: true,
            // attribute index
            custom_attribute_index_buffers: DynamicStorageBuffer::new(
                Self::INDICES_INITIAL_SIZE,
                Some("MeshAttributeIndex".to_string()),
            ),
            custom_attribute_index_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MeshAttributeIndex"),
                    Self::INDICES_INITIAL_SIZE,
                    BufferUsage::new().with_copy_dst().with_storage(),
                )
                .into(),
            )?,
            custom_attribute_index_dirty: true,
            meta: MeshMeta::new(gpu)?,
            // attribute morphs and skins
            morphs: Morphs::new(gpu)?,
            skins: Skins::new(gpu)?,
        })
    }

    pub fn insert(
        &mut self,
        mesh: Mesh,
        materials: &Materials,
        transforms: &Transforms,
        visibility_geometry_data: Option<&[u8]>,
        transparency_geometry_data: Option<&[u8]>,
        attribute_data: &[u8],
        attribute_index: &[u8],
    ) -> Result<MeshKey> {
        let transform_key = mesh.transform_key;
        let buffer_info_key = mesh.buffer_info_key;
        let mesh_key = self.list.insert(mesh.clone());

        let buffer_info = self.buffer_infos.get(buffer_info_key)?;

        self.transform_to_meshes
            .entry(transform_key)
            .unwrap()
            .or_default()
            .push(mesh_key);

        // geometry

        let visibility_geometry_data_offset = match visibility_geometry_data {
            Some(geometry_data) => {
                // visibility geometry - index (auto-generated sequential for drawing)
                if let Some(vertex_info) = &buffer_info.visibility_geometry_vertex {
                    let mut geometry_index = Vec::new();
                    for i in 0..vertex_info.count {
                        geometry_index.extend_from_slice(&(i as u32).to_le_bytes());
                    }
                    self.visibility_geometry_index_buffers
                        .update(mesh_key, &geometry_index);
                } else {
                    return Err(AwsmMeshError::VisibilityGeometryBufferInfoNotFound(
                        buffer_info_key,
                    ));
                }

                self.visibility_geometry_index_dirty = true;
                let offset = self
                    .visibility_geometry_data_buffers
                    .update(mesh_key, geometry_data);
                self.visibility_geometry_data_dirty = true;

                Some(offset)
            }
            None => None,
        };

        let transparency_geometry_data_offset = match transparency_geometry_data {
            Some(geometry_data) => {
                let offset = self
                    .transparency_geometry_data_buffers
                    .update(mesh_key, geometry_data);
                self.transparency_geometry_data_dirty = true;

                Some(offset)
            }
            None => None,
        };

        // attributes - index
        let custom_attribute_indices_offset = self
            .custom_attribute_index_buffers
            .update(mesh_key, attribute_index);
        self.custom_attribute_index_dirty = true;

        // attributes - data
        let custom_attribute_data_offset = self
            .custom_attribute_data_buffers
            .update(mesh_key, attribute_data);
        self.custom_attribute_data_dirty = true;

        // KEEP THIS AROUND FOR DEBUGGING
        // Very helpful - shows all the non-position vertex attributes and triangle indices
        // tracing::info!(
        //     "attribute indices: {:?}",
        //     buffer_info
        //         .triangles
        //         .vertex_attribute_indices
        //         .debug_to_vec(attribute_index)
        // );
        // for attr in buffer_info.triangles.vertex_attributes.iter() {
        //     tracing::info!(
        //         "attribute data {:?}: {:?}",
        //         attr,
        //         buffer_info
        //             .triangles
        //             .debug_get_attribute_vec_f32(attr, attribute_data)
        //     );
        // }

        // for attr in buffer_info.triangles.vertex_attributes.iter() {
        //     match attr {
        //         crate::mesh::MeshBufferVertexAttributeInfo::Custom(
        //             crate::mesh::MeshBufferCustomVertexAttributeInfo::Colors { .. },
        //         ) => {
        //             tracing::info!(
        //                 "attribute data {:?}: {:?}",
        //                 attr,
        //                 buffer_info
        //                     .triangles
        //                     .debug_get_attribute_vec_f32(attr, attribute_data)
        //             );
        //         }
        //         _ => {}
        //     }
        // }
        self.meta.insert(
            mesh_key,
            &mesh,
            buffer_info,
            visibility_geometry_data_offset,
            transparency_geometry_data_offset,
            custom_attribute_indices_offset,
            custom_attribute_data_offset,
            materials,
            transforms,
            &self.morphs,
            &self.skins,
        )?;

        Ok(mesh_key)
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

    pub fn keys_by_transform_key(&self, transform_key: TransformKey) -> Option<&Vec<MeshKey>> {
        self.transform_to_meshes.get(transform_key)
    }

    pub fn keys(&self) -> impl Iterator<Item = MeshKey> + '_ {
        self.list.keys()
    }

    pub fn visibility_geometry_data_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.visibility_geometry_data_gpu_buffer
    }
    pub fn visibility_geometry_data_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.visibility_geometry_data_buffers
            .offset(key)
            .ok_or(AwsmMeshError::VisibilityGeometryBufferNotFound(key))
    }

    pub fn visibility_geometry_index_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.visibility_geometry_index_gpu_buffer
    }
    pub fn visibility_geometry_index_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.visibility_geometry_index_buffers
            .offset(key)
            .ok_or(AwsmMeshError::VisibilityGeometryBufferNotFound(key))
    }

    pub fn custom_attribute_data_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.custom_attribute_data_gpu_buffer
    }
    pub fn custom_attribute_data_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.custom_attribute_data_buffers
            .offset(key)
            .ok_or(AwsmMeshError::CustomAttributeBufferNotFound(key))
    }

    pub fn transparency_geometry_data_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.transparency_geometry_data_gpu_buffer
    }
    pub fn transparency_geometry_data_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.transparency_geometry_data_buffers
            .offset(key)
            .ok_or(AwsmMeshError::TransparencyGeometryBufferNotFound(key))
    }
    // re-use the custom attribute index methods
    pub fn transparency_geometry_index_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.custom_attribute_index_gpu_buffer
    }
    pub fn transparency_geometry_index_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.custom_attribute_index_buffers
            .offset(key)
            .ok_or(AwsmMeshError::CustomAttributeBufferNotFound(key))
    }

    pub fn custom_attribute_index_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.custom_attribute_index_gpu_buffer
    }
    pub fn custom_attribute_index_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.custom_attribute_index_buffers
            .offset(key)
            .ok_or(AwsmMeshError::CustomAttributeBufferNotFound(key))
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

    pub fn remove_by_transform_key(&mut self, transform_key: TransformKey) -> Option<Vec<Mesh>> {
        if let Some(mesh_keys) = self.transform_to_meshes.get(transform_key).cloned() {
            let mut removed_meshes = Vec::with_capacity(mesh_keys.capacity());
            for mesh_key in mesh_keys.iter() {
                if let Some(mesh) = self.remove(*mesh_key) {
                    removed_meshes.push(mesh);
                }
            }
            Some(removed_meshes)
        } else {
            None
        }
    }
    pub fn remove(&mut self, mesh_key: MeshKey) -> Option<Mesh> {
        if let Some(mesh) = self.list.remove(mesh_key) {
            self.visibility_geometry_data_buffers.remove(mesh_key);
            self.visibility_geometry_index_buffers.remove(mesh_key);
            self.transparency_geometry_data_buffers.remove(mesh_key);
            self.custom_attribute_data_buffers.remove(mesh_key);
            self.custom_attribute_index_buffers.remove(mesh_key);
            self.meta.remove(mesh_key);

            if self.buffer_infos.remove(mesh.buffer_info_key).is_some() {
                self.visibility_geometry_data_dirty = true;
                self.visibility_geometry_index_dirty = true;
                self.transparency_geometry_data_dirty = true;
                self.custom_attribute_data_dirty = true;
                self.custom_attribute_index_dirty = true;
            }

            if let Some(meshes) = self.transform_to_meshes.get_mut(mesh.transform_key) {
                meshes.retain(|&key| key != mesh_key)
            }

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
                self.visibility_geometry_data_dirty,
                &mut self.visibility_geometry_data_buffers,
                &mut self.visibility_geometry_data_gpu_buffer,
                BufferUsage::new()
                    .with_copy_dst()
                    .with_vertex()
                    .with_storage(),
                "MeshVisibilityGeometryData",
                None,
            ),
            (
                self.visibility_geometry_index_dirty,
                &mut self.visibility_geometry_index_buffers,
                &mut self.visibility_geometry_index_gpu_buffer,
                BufferUsage::new().with_copy_dst().with_index(),
                "MeshVisibilityIndex",
                None,
            ),
            (
                self.transparency_geometry_data_dirty,
                &mut self.transparency_geometry_data_buffers,
                &mut self.transparency_geometry_data_gpu_buffer,
                BufferUsage::new()
                    .with_copy_dst()
                    .with_vertex()
                    .with_storage(),
                "MeshTransparencyGeometryData",
                None,
            ),
            (
                self.custom_attribute_data_dirty,
                &mut self.custom_attribute_data_buffers,
                &mut self.custom_attribute_data_gpu_buffer,
                BufferUsage::new()
                    .with_copy_dst()
                    .with_storage()
                    .with_vertex(),
                "MeshCustomAttributeData",
                Some(BindGroupCreate::MeshAttributeDataResize),
            ),
            (
                self.custom_attribute_index_dirty,
                &mut self.custom_attribute_index_buffers,
                &mut self.custom_attribute_index_gpu_buffer,
                BufferUsage::new()
                    .with_copy_dst()
                    .with_storage()
                    .with_index(),
                "MeshCustomAttributeIndex",
                Some(BindGroupCreate::MeshAttributeIndexResize),
            ),
        ];

        let any_dirty = to_check_dynamic.iter().any(|(dirty, _, _, _, _, _)| *dirty);

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
                    gpu.write_buffer(gpu_buffer, None, buffer.raw_slice(), None, None)?;
                }
            }

            self.visibility_geometry_data_dirty = false;
            self.visibility_geometry_index_dirty = false;
            self.transparency_geometry_data_dirty = false;
            self.custom_attribute_data_dirty = false;
            self.custom_attribute_index_dirty = false;
        }

        Ok(())
    }
}

impl Drop for Meshes {
    fn drop(&mut self) {
        self.visibility_geometry_data_gpu_buffer.destroy();
        self.visibility_geometry_index_gpu_buffer.destroy();
        self.transparency_geometry_data_gpu_buffer.destroy();
        self.custom_attribute_data_gpu_buffer.destroy();
        self.custom_attribute_index_gpu_buffer.destroy();
    }
}

new_key_type! {
    pub struct MeshKey;
}
