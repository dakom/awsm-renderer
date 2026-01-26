//! Mesh storage and GPU buffer management.

use std::collections::HashMap;

use awsm_renderer_core::buffers::{BufferDescriptor, BufferUsage};
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::Mat4;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::bounds::Aabb;
use crate::bind_groups::{BindGroupCreate, BindGroups};
use crate::buffer::dynamic_storage::DynamicStorageBuffer;
use crate::buffer::helpers::write_buffer_with_dirty_ranges;
use crate::instances::Instances;
use crate::materials::Materials;
use crate::mesh::meta::{MeshMeta, MESH_META_INITIAL_CAPACITY};
use crate::mesh::skins::{SkinKey, Skins};
use crate::mesh::{MeshBufferInfoKey, MeshBufferInfos};
use crate::transforms::{Transform, TransformKey, Transforms};
use crate::AwsmRendererLogging;

use super::error::{AwsmMeshError, Result};
use super::morphs::{GeometryMorphKey, MaterialMorphKey, Morphs};
use super::{Mesh, MeshBufferVertexInfo};

/// Shared mesh resource data and buffer offsets.
#[derive(Debug, Clone)]
pub struct MeshResource {
    pub buffer_info_key: MeshBufferInfoKey,
    pub visibility_geometry_data_offset: Option<usize>,
    pub transparency_geometry_data_offset: Option<usize>,
    pub custom_attribute_data_offset: usize,
    pub custom_attribute_index_offset: usize,
    pub aabb: Option<Aabb>,
    pub geometry_morph_key: Option<GeometryMorphKey>,
    pub material_morph_key: Option<MaterialMorphKey>,
    pub skin_key: Option<SkinKey>,
    pub refcount: usize,
}

/// Mesh list with shared resources and GPU buffers.
pub struct Meshes {
    list: DenseSlotMap<MeshKey, Mesh>,
    resources: DenseSlotMap<MeshResourceKey, MeshResource>,
    mesh_to_resource: SecondaryMap<MeshKey, MeshResourceKey>,
    transform_to_meshes: SecondaryMap<TransformKey, Vec<MeshKey>>,
    // visibility geometry data buffers (position, triangle-id, barycentric)
    visibility_geometry_data_buffers: DynamicStorageBuffer<MeshResourceKey>,
    visibility_geometry_data_gpu_buffer: web_sys::GpuBuffer,
    visibility_geometry_data_dirty: bool,
    // visibility geometry index buffers (position, triangle-id, barycentric, etc.)
    visibility_geometry_index_buffers: DynamicStorageBuffer<MeshResourceKey>,
    visibility_geometry_index_gpu_buffer: web_sys::GpuBuffer,
    visibility_geometry_index_dirty: bool,
    // transparency geometry data buffers (position, etc.)
    transparency_geometry_data_buffers: DynamicStorageBuffer<MeshResourceKey>,
    transparency_geometry_data_gpu_buffer: web_sys::GpuBuffer,
    transparency_geometry_data_dirty: bool,
    // attribute data buffers
    custom_attribute_data_buffers: DynamicStorageBuffer<MeshResourceKey>,
    custom_attribute_data_gpu_buffer: web_sys::GpuBuffer,
    custom_attribute_data_dirty: bool,
    // attribute index buffers (normals, uvs, colors, etc.)
    custom_attribute_index_buffers: DynamicStorageBuffer<MeshResourceKey>,
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

    /// Creates mesh storage and GPU buffers.
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            list: DenseSlotMap::with_key(),
            resources: DenseSlotMap::with_key(),
            mesh_to_resource: SecondaryMap::new(),
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

    /// Inserts a mesh and its backing resource data, returning a mesh key.
    pub fn insert(
        &mut self,
        mesh: Mesh,
        materials: &Materials,
        transforms: &Transforms,
        buffer_info_key: MeshBufferInfoKey,
        visibility_geometry_data: Option<&[u8]>,
        transparency_geometry_data: Option<&[u8]>,
        attribute_data: &[u8],
        attribute_index: &[u8],
        aabb: Option<Aabb>,
        geometry_morph_key: Option<GeometryMorphKey>,
        material_morph_key: Option<MaterialMorphKey>,
        skin_key: Option<SkinKey>,
    ) -> Result<MeshKey> {
        let resource_key = self.insert_resource(
            buffer_info_key,
            visibility_geometry_data,
            transparency_geometry_data,
            attribute_data,
            attribute_index,
            aabb,
            geometry_morph_key,
            material_morph_key,
            skin_key,
        )?;

        self.insert_instance(mesh, resource_key, materials, transforms)
    }

    fn insert_resource(
        &mut self,
        buffer_info_key: MeshBufferInfoKey,
        visibility_geometry_data: Option<&[u8]>,
        transparency_geometry_data: Option<&[u8]>,
        attribute_data: &[u8],
        attribute_index: &[u8],
        aabb: Option<Aabb>,
        geometry_morph_key: Option<GeometryMorphKey>,
        material_morph_key: Option<MaterialMorphKey>,
        skin_key: Option<SkinKey>,
    ) -> Result<MeshResourceKey> {
        let buffer_info = self.buffer_infos.get(buffer_info_key)?;

        let resource_key = self.resources.insert(MeshResource {
            buffer_info_key,
            visibility_geometry_data_offset: None,
            transparency_geometry_data_offset: None,
            custom_attribute_data_offset: 0,
            custom_attribute_index_offset: 0,
            aabb,
            geometry_morph_key,
            material_morph_key,
            skin_key,
            refcount: 1,
        });

        let visibility_geometry_data_offset = match visibility_geometry_data {
            Some(geometry_data) => {
                if let Some(vertex_info) = &buffer_info.visibility_geometry_vertex {
                    let mut geometry_index = Vec::new();
                    for i in 0..vertex_info.count {
                        geometry_index.extend_from_slice(&(i as u32).to_le_bytes());
                    }
                    self.visibility_geometry_index_buffers
                        .update(resource_key, &geometry_index);
                } else {
                    return Err(AwsmMeshError::VisibilityGeometryBufferInfoNotFound(
                        buffer_info_key,
                    ));
                }

                self.visibility_geometry_index_dirty = true;
                let offset = self
                    .visibility_geometry_data_buffers
                    .update(resource_key, geometry_data);
                self.visibility_geometry_data_dirty = true;

                Some(offset)
            }
            None => None,
        };

        let transparency_geometry_data_offset = match transparency_geometry_data {
            Some(geometry_data) => {
                let offset = self
                    .transparency_geometry_data_buffers
                    .update(resource_key, geometry_data);
                self.transparency_geometry_data_dirty = true;

                Some(offset)
            }
            None => None,
        };

        let custom_attribute_indices_offset = self
            .custom_attribute_index_buffers
            .update(resource_key, attribute_index);
        self.custom_attribute_index_dirty = true;

        let custom_attribute_data_offset = self
            .custom_attribute_data_buffers
            .update(resource_key, attribute_data);
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

        if let Some(resource) = self.resources.get_mut(resource_key) {
            resource.visibility_geometry_data_offset = visibility_geometry_data_offset;
            resource.transparency_geometry_data_offset = transparency_geometry_data_offset;
            resource.custom_attribute_data_offset = custom_attribute_data_offset;
            resource.custom_attribute_index_offset = custom_attribute_indices_offset;
        }

        Ok(resource_key)
    }

    fn insert_instance(
        &mut self,
        mut mesh: Mesh,
        resource_key: MeshResourceKey,
        materials: &Materials,
        transforms: &Transforms,
    ) -> Result<MeshKey> {
        let transform_key = mesh.transform_key;

        let (
            resource_aabb,
            buffer_info_key,
            visibility_geometry_data_offset,
            custom_attribute_index_offset,
            custom_attribute_data_offset,
            geometry_morph_key,
            material_morph_key,
            skin_key,
        ) = {
            let resource = self
                .resources
                .get(resource_key)
                .ok_or(AwsmMeshError::ResourceNotFound(resource_key))?;

            (
                resource.aabb.clone(),
                resource.buffer_info_key,
                resource.visibility_geometry_data_offset,
                resource.custom_attribute_index_offset,
                resource.custom_attribute_data_offset,
                resource.geometry_morph_key,
                resource.material_morph_key,
                resource.skin_key,
            )
        };

        if mesh.world_aabb.is_none() {
            mesh.world_aabb = resource_aabb;
        }

        let mesh_key = self.list.insert(mesh.clone());
        self.mesh_to_resource.insert(mesh_key, resource_key);

        self.transform_to_meshes
            .entry(transform_key)
            .unwrap()
            .or_default()
            .push(mesh_key);

        let buffer_info = self.buffer_infos.get(buffer_info_key)?;

        self.meta.insert(
            mesh_key,
            &mesh,
            buffer_info,
            visibility_geometry_data_offset,
            custom_attribute_index_offset,
            custom_attribute_data_offset,
            geometry_morph_key,
            material_morph_key,
            skin_key,
            materials,
            transforms,
            &self.morphs,
            &self.skins,
        )?;

        Ok(mesh_key)
    }

    /// Duplicates a mesh instance and assigns a new transform key.
    pub fn duplicate_with_transform(
        &mut self,
        mesh_key: MeshKey,
        new_transform_key: TransformKey,
        materials: &Materials,
        transforms: &Transforms,
    ) -> Result<MeshKey> {
        let mesh = self.get(mesh_key)?.clone();
        let resource_key = self.resource_key(mesh_key)?;
        let resource_aabb = {
            let resource = self
                .resources
                .get_mut(resource_key)
                .ok_or(AwsmMeshError::ResourceNotFound(resource_key))?;
            resource.refcount += 1;
            resource.aabb.clone()
        };

        let mut new_mesh = mesh.clone();
        new_mesh.transform_key = new_transform_key;
        new_mesh.world_aabb = resource_aabb;

        self.insert_instance(new_mesh, resource_key, materials, transforms)
    }

    /// Duplicates all meshes under a transform into a new transform key.
    pub fn duplicate_by_transform_key(
        &mut self,
        transform_key: TransformKey,
        materials: &Materials,
        transforms: &mut Transforms,
    ) -> Result<(TransformKey, Vec<MeshKey>)> {
        let mesh_keys = self
            .transform_to_meshes
            .get(transform_key)
            .cloned()
            .ok_or(AwsmMeshError::TransformHasNoMeshes(transform_key))?;

        if mesh_keys.is_empty() {
            return Err(AwsmMeshError::TransformHasNoMeshes(transform_key));
        }

        for mesh_key in &mesh_keys {
            if self.get(*mesh_key)?.instanced {
                return Err(AwsmMeshError::InstancedMeshUnsupported(*mesh_key));
            }
        }

        let new_transform_key = transforms.duplicate(transform_key)?;

        let mut new_mesh_keys = Vec::with_capacity(mesh_keys.len());
        for mesh_key in mesh_keys {
            let new_mesh_key =
                self.duplicate_with_transform(mesh_key, new_transform_key, materials, transforms)?;
            new_mesh_keys.push(new_mesh_key);
        }

        Ok((new_transform_key, new_mesh_keys))
    }

    /// Splits a mesh into a new transform key so it can move independently.
    pub fn split_mesh(
        &mut self,
        mesh_key: MeshKey,
        transforms: &mut Transforms,
        materials: &Materials,
    ) -> Result<TransformKey> {
        let old_transform_key = self.get(mesh_key)?.transform_key;
        if self.get(mesh_key)?.instanced {
            return Err(AwsmMeshError::InstancedMeshUnsupported(mesh_key));
        }

        let new_transform_key = transforms.duplicate(old_transform_key)?;

        self.update_mesh_transform(mesh_key, old_transform_key, new_transform_key, materials, transforms)?;

        Ok(new_transform_key)
    }

    /// Splits all meshes under a transform into independent transforms.
    pub fn split_meshes_by_transform_key(
        &mut self,
        transform_key: TransformKey,
        transforms: &mut Transforms,
        materials: &Materials,
    ) -> Result<Vec<(MeshKey, TransformKey)>> {
        let mesh_keys = self
            .transform_to_meshes
            .get(transform_key)
            .cloned()
            .ok_or(AwsmMeshError::TransformHasNoMeshes(transform_key))?;

        if mesh_keys.is_empty() {
            return Err(AwsmMeshError::TransformHasNoMeshes(transform_key));
        }

        let mut out = Vec::with_capacity(mesh_keys.len());
        for mesh_key in mesh_keys {
            let new_transform_key = self.split_mesh(mesh_key, transforms, materials)?;
            out.push((mesh_key, new_transform_key));
        }

        Ok(out)
    }

    /// Joins multiple meshes under a single transform key.
    pub fn join_meshes(
        &mut self,
        mesh_keys: &[MeshKey],
        transforms: &mut Transforms,
        materials: &Materials,
        transform_override: Option<Transform>,
    ) -> Result<(TransformKey, Vec<MeshKey>)> {
        if mesh_keys.is_empty() {
            return Err(AwsmMeshError::MeshListEmpty);
        }

        for mesh_key in mesh_keys {
            if self.get(*mesh_key)?.instanced {
                return Err(AwsmMeshError::InstancedMeshUnsupported(*mesh_key));
            }
        }

        let mut common_parent = None;
        for (index, mesh_key) in mesh_keys.iter().enumerate() {
            let mesh = self.get(*mesh_key)?;
            let parent = transforms.get_parent(mesh.transform_key).ok();
            if index == 0 {
                common_parent = parent;
            } else if common_parent != parent {
                common_parent = None;
                break;
            }
        }

        let new_local = match transform_override {
            Some(transform) => transform,
            None => {
                let mut center_sum = glam::Vec3::ZERO;
                for mesh_key in mesh_keys {
                    let mesh = self.get(*mesh_key)?;
                    let center = mesh
                        .world_aabb
                        .as_ref()
                        .map(|aabb| aabb.center())
                        .or_else(|| transforms.get_world(mesh.transform_key).ok().map(|m| m.w_axis.truncate()))
                        .unwrap_or(glam::Vec3::ZERO);
                    center_sum += center;
                }
                let centroid_world = center_sum / mesh_keys.len() as f32;
                let local_translation = match common_parent {
                    Some(parent_key) => transforms
                        .get_world(parent_key)
                        .ok()
                        .map(|m| m.inverse().transform_point3(centroid_world))
                        .unwrap_or(centroid_world),
                    None => centroid_world,
                };
                Transform::IDENTITY.with_translation(local_translation)
            }
        };

        let new_transform_key = transforms.insert(new_local, common_parent);

        let moved = mesh_keys.to_vec();
        for mesh_key in &moved {
            let old_transform_key = self.get(*mesh_key)?.transform_key;
            self.update_mesh_transform(*mesh_key, old_transform_key, new_transform_key, materials, transforms)?;
        }

        Ok((new_transform_key, moved))
    }

    /// Updates world-space AABBs for meshes affected by dirty transforms or instances.
    pub fn update_world(
        &mut self,
        dirty_transforms: HashMap<TransformKey, Mat4>,
        dirty_instances: &std::collections::HashSet<TransformKey>,
        transforms: &Transforms,
        instances: &Instances,
    ) {
        let mut update_keys = std::collections::HashSet::new();
        update_keys.extend(dirty_transforms.keys().copied());
        update_keys.extend(dirty_instances.iter().copied());

        // This doesn't mark anything as dirty, it just updates the world AABB for frustum culling and depth sorting
        for transform_key in update_keys {
            let world_mat = dirty_transforms
                .get(&transform_key)
                .copied()
                .or_else(|| transforms.get_world(transform_key).ok().copied());

            let world_mat = match world_mat {
                Some(mat) => mat,
                None => continue,
            };

            if let Some(mesh_keys) = self.transform_to_meshes.get(transform_key) {
                for mesh_key in mesh_keys {
                    let resource_aabb = self
                        .resource(*mesh_key)
                        .ok()
                        .and_then(|resource| resource.aabb.clone());

                    let world_aabb = match resource_aabb {
                        Some(aabb) => {
                            let mesh = match self.list.get(*mesh_key) {
                                Some(mesh) => mesh,
                                None => continue,
                            };

                            if mesh.instanced {
                                match instances.transform_list(mesh.transform_key) {
                                    Some(transforms_list) if !transforms_list.is_empty() => {
                                        let first = world_mat * transforms_list[0].to_matrix();
                                        let mut combined = aabb.transformed(&first);
                                        for transform in &transforms_list[1..] {
                                            let world = world_mat * transform.to_matrix();
                                            let transformed = aabb.transformed(&world);
                                            combined.extend(&transformed);
                                        }
                                        Some(combined)
                                    }
                                    _ => None,
                                }
                            } else {
                                Some(aabb.transformed(&world_mat))
                            }
                        }
                        None => None,
                    };

                    if let Some(mesh) = self.list.get_mut(*mesh_key) {
                        mesh.world_aabb = world_aabb;
                    }
                }
            }
        }

        // This does update the GPU as dirty, bit skins manage their own GPU dirty state
        self.skins.update_transforms(dirty_transforms);
    }

    fn update_mesh_transform(
        &mut self,
        mesh_key: MeshKey,
        old_transform_key: TransformKey,
        new_transform_key: TransformKey,
        materials: &Materials,
        transforms: &Transforms,
    ) -> Result<()> {
        let resource_aabb = self.resource(mesh_key).ok().and_then(|r| r.aabb.clone());

        if let Some(mesh) = self.list.get_mut(mesh_key) {
            mesh.transform_key = new_transform_key;
            mesh.world_aabb = resource_aabb;
        }

        if let Some(meshes) = self.transform_to_meshes.get_mut(old_transform_key) {
            meshes.retain(|&key| key != mesh_key);
        }
        if let Some(meshes) = self.transform_to_meshes.get(old_transform_key) {
            if meshes.is_empty() {
                self.transform_to_meshes.remove(old_transform_key);
            }
        }

        if let Some(meshes) = self.transform_to_meshes.get_mut(new_transform_key) {
            meshes.push(mesh_key);
        } else {
            self.transform_to_meshes
                .insert(new_transform_key, vec![mesh_key]);
        }

        self.refresh_meta_for_mesh(mesh_key, materials, transforms)?;

        Ok(())
    }

    fn refresh_meta_for_mesh(
        &mut self,
        mesh_key: MeshKey,
        materials: &Materials,
        transforms: &Transforms,
    ) -> Result<()> {
        let mesh = self
            .list
            .get(mesh_key)
            .ok_or(AwsmMeshError::MeshNotFound(mesh_key))?;

        let (
            buffer_info_key,
            visibility_geometry_data_offset,
            custom_attribute_index_offset,
            custom_attribute_data_offset,
            geometry_morph_key,
            material_morph_key,
            skin_key,
        ) = {
            let resource = self.resource(mesh_key)?;
            (
                resource.buffer_info_key,
                resource.visibility_geometry_data_offset,
                resource.custom_attribute_index_offset,
                resource.custom_attribute_data_offset,
                resource.geometry_morph_key,
                resource.material_morph_key,
                resource.skin_key,
            )
        };

        let buffer_info = self.buffer_infos.get(buffer_info_key)?;

        self.meta.insert(
            mesh_key,
            mesh,
            buffer_info,
            visibility_geometry_data_offset,
            custom_attribute_index_offset,
            custom_attribute_data_offset,
            geometry_morph_key,
            material_morph_key,
            skin_key,
            materials,
            transforms,
            &self.morphs,
            &self.skins,
        )?;

        Ok(())
    }

    /// Returns mesh keys associated with a transform key.
    pub fn keys_by_transform_key(&self, transform_key: TransformKey) -> Option<&Vec<MeshKey>> {
        self.transform_to_meshes.get(transform_key)
    }

    /// Iterates over all mesh keys.
    pub fn keys(&self) -> impl Iterator<Item = MeshKey> + '_ {
        self.list.keys()
    }

    /// Returns the resource key for a mesh.
    pub fn resource_key(&self, mesh_key: MeshKey) -> Result<MeshResourceKey> {
        self.mesh_to_resource
            .get(mesh_key)
            .copied()
            .ok_or(AwsmMeshError::MeshNotFound(mesh_key))
    }

    /// Returns the buffer info key for a mesh.
    pub fn buffer_info_key(&self, mesh_key: MeshKey) -> Result<MeshBufferInfoKey> {
        Ok(self.resource(mesh_key)?.buffer_info_key)
    }

    /// Returns the buffer info for a mesh.
    pub fn buffer_info(&self, mesh_key: MeshKey) -> Result<&crate::mesh::MeshBufferInfo> {
        let buffer_info_key = self.buffer_info_key(mesh_key)?;
        self.buffer_infos.get(buffer_info_key)
    }

    /// Returns the mesh resource referenced by a mesh key.
    pub fn resource(&self, mesh_key: MeshKey) -> Result<&MeshResource> {
        let resource_key = self.resource_key(mesh_key)?;
        self.resources
            .get(resource_key)
            .ok_or(AwsmMeshError::ResourceNotFound(resource_key))
    }

    /// Returns the GPU buffer for visibility geometry vertex data.
    pub fn visibility_geometry_data_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.visibility_geometry_data_gpu_buffer
    }
    /// Returns the offset into visibility geometry data for a mesh.
    pub fn visibility_geometry_data_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        let resource_key = self.resource_key(key)?;
        self.visibility_geometry_data_buffers
            .offset(resource_key)
            .ok_or(AwsmMeshError::VisibilityGeometryBufferNotFound(key))
    }

    /// Returns the GPU buffer for visibility geometry indices.
    pub fn visibility_geometry_index_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.visibility_geometry_index_gpu_buffer
    }
    /// Returns the offset into visibility geometry indices for a mesh.
    pub fn visibility_geometry_index_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        let resource_key = self.resource_key(key)?;
        self.visibility_geometry_index_buffers
            .offset(resource_key)
            .ok_or(AwsmMeshError::VisibilityGeometryBufferNotFound(key))
    }

    /// Returns the GPU buffer for custom attribute vertex data.
    pub fn custom_attribute_data_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.custom_attribute_data_gpu_buffer
    }
    /// Returns the offset into custom attribute vertex data for a mesh.
    pub fn custom_attribute_data_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        let resource_key = self.resource_key(key)?;
        self.custom_attribute_data_buffers
            .offset(resource_key)
            .ok_or(AwsmMeshError::CustomAttributeBufferNotFound(key))
    }

    /// Returns the GPU buffer for transparency geometry vertex data.
    pub fn transparency_geometry_data_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.transparency_geometry_data_gpu_buffer
    }
    /// Returns the offset into transparency geometry data for a mesh.
    pub fn transparency_geometry_data_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        let resource_key = self.resource_key(key)?;
        self.transparency_geometry_data_buffers
            .offset(resource_key)
            .ok_or(AwsmMeshError::TransparencyGeometryBufferNotFound(key))
    }
    // re-use the custom attribute index methods
    /// Returns the GPU buffer for transparency geometry indices.
    pub fn transparency_geometry_index_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.custom_attribute_index_gpu_buffer
    }
    /// Returns the offset into transparency geometry indices for a mesh.
    pub fn transparency_geometry_index_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        let resource_key = self.resource_key(key)?;
        self.custom_attribute_index_buffers
            .offset(resource_key)
            .ok_or(AwsmMeshError::CustomAttributeBufferNotFound(key))
    }

    /// Returns the GPU buffer for custom attribute indices.
    pub fn custom_attribute_index_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.custom_attribute_index_gpu_buffer
    }
    /// Returns the offset into custom attribute indices for a mesh.
    pub fn custom_attribute_index_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        let resource_key = self.resource_key(key)?;
        self.custom_attribute_index_buffers
            .offset(resource_key)
            .ok_or(AwsmMeshError::CustomAttributeBufferNotFound(key))
    }

    /// Iterates over meshes and their keys.
    pub fn iter(&self) -> impl Iterator<Item = (MeshKey, &Mesh)> {
        self.list.iter()
    }

    /// Returns a mesh by key.
    pub fn get(&self, mesh_key: MeshKey) -> Result<&Mesh> {
        self.list
            .get(mesh_key)
            .ok_or(AwsmMeshError::MeshNotFound(mesh_key))
    }

    /// Returns a mutable mesh by key.
    pub fn get_mut(&mut self, mesh_key: MeshKey) -> Result<&mut Mesh> {
        self.list
            .get_mut(mesh_key)
            .ok_or(AwsmMeshError::MeshNotFound(mesh_key))
    }

    /// Removes all meshes that share the given transform key.
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
    /// Removes a mesh by key and returns it if found.
    pub fn remove(&mut self, mesh_key: MeshKey) -> Option<Mesh> {
        if let Some(mesh) = self.list.remove(mesh_key) {
            self.meta.remove(mesh_key);

            if let Some(meshes) = self.transform_to_meshes.get_mut(mesh.transform_key) {
                meshes.retain(|&key| key != mesh_key)
            }

            if let Some(resource_key) = self.mesh_to_resource.remove(mesh_key) {
                let should_remove_resource = match self.resources.get_mut(resource_key) {
                    Some(resource) => {
                        if resource.refcount > 1 {
                            resource.refcount -= 1;
                            false
                        } else {
                            true
                        }
                    }
                    None => false,
                };

                if should_remove_resource {
                    if let Some(resource) = self.resources.remove(resource_key) {
                        self.visibility_geometry_data_buffers.remove(resource_key);
                        self.visibility_geometry_index_buffers.remove(resource_key);
                        self.transparency_geometry_data_buffers.remove(resource_key);
                        self.custom_attribute_data_buffers.remove(resource_key);
                        self.custom_attribute_index_buffers.remove(resource_key);

                        self.visibility_geometry_data_dirty = true;
                        self.visibility_geometry_index_dirty = true;
                        self.transparency_geometry_data_dirty = true;
                        self.custom_attribute_data_dirty = true;
                        self.custom_attribute_index_dirty = true;

                        if self.buffer_infos.remove(resource.buffer_info_key).is_some() {
                            self.visibility_geometry_data_dirty = true;
                            self.visibility_geometry_index_dirty = true;
                            self.transparency_geometry_data_dirty = true;
                            self.custom_attribute_data_dirty = true;
                            self.custom_attribute_index_dirty = true;
                        }

                        if let Some(morph_key) = resource.geometry_morph_key {
                            self.morphs.geometry.remove(morph_key);
                        }

                        if let Some(morph_key) = resource.material_morph_key {
                            self.morphs.material.remove(morph_key);
                        }

                        if let Some(skin_key) = resource.skin_key {
                            self.skins.remove(skin_key, None);
                        }
                    }
                }
            }

            Some(mesh)
        } else {
            None
        }
    }

    /// Writes dirty mesh buffers to the GPU and updates bind groups.
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
                    let mut resized = false;
                    if let Some(new_size) = buffer.take_gpu_needs_resize() {
                        *gpu_buffer = gpu.create_buffer(
                            &BufferDescriptor::new(Some(label), new_size, usage).into(),
                        )?;

                        if let Some(create) = bind_group_create {
                            bind_groups.mark_create(create);
                        }
                        resized = true;
                    }
                    if resized {
                        buffer.clear_dirty_ranges();
                        gpu.write_buffer(gpu_buffer, None, buffer.raw_slice(), None, None)?;
                    } else {
                        let ranges = buffer.take_dirty_ranges();
                        write_buffer_with_dirty_ranges(gpu, gpu_buffer, buffer.raw_slice(), ranges)?;
                    }
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
    /// Opaque key for mesh instances.
    pub struct MeshKey;
    /// Opaque key for shared mesh resources.
    pub struct MeshResourceKey;
}
