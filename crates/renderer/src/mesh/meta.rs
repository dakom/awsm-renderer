use awsm_renderer_core::{
    buffers::{BufferDescriptor, BufferUsage},
    renderer::AwsmRendererWebGpu,
};
use slotmap::Key;

use crate::{
    bind_groups::{BindGroupCreate, BindGroups},
    buffer::dynamic_uniform::DynamicUniformBuffer,
    debug::AwsmRendererLogging,
    materials::{MaterialKey, Materials},
    mesh::{
        error::{AwsmMeshError, Result},
        morphs::{GeometryMorphKey, MaterialMorphKey, Morphs},
        skins::{SkinKey, Skins},
        Mesh, MeshBufferInfo, MeshKey,
    },
    transforms::{TransformKey, Transforms},
};

pub const MESH_META_INITIAL_CAPACITY: usize = 1024;
pub const GEOMETRY_MESH_META_BYTE_SIZE: usize = 40;
pub const GEOMETRY_MESH_META_BYTE_ALIGNMENT: usize = 256; // 32 bytes aligned
pub const MATERIAL_MESH_META_MORPH_MATERIAL_BITMASK_NORMAL: u32 = 1;
pub const MATERIAL_MESH_META_MORPH_MATERIAL_BITMASK_TANGENT: u32 = 1 << 1;
pub const MATERIAL_MESH_META_BYTE_SIZE: usize = 48;
pub const MATERIAL_MESH_META_BYTE_ALIGNMENT: usize = 256; // 32 bytes aligned

pub struct MeshMeta {
    // meta data buffers
    geometry_buffers: DynamicUniformBuffer<MeshKey>,
    geometry_gpu_buffer: web_sys::GpuBuffer,
    geometry_dirty: bool,
    // meta data buffers
    material_buffers: DynamicUniformBuffer<MeshKey>,
    material_gpu_buffer: web_sys::GpuBuffer,
    material_dirty: bool,
}

impl MeshMeta {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            geometry_buffers: DynamicUniformBuffer::new(
                MESH_META_INITIAL_CAPACITY,
                GEOMETRY_MESH_META_BYTE_SIZE,
                Some(GEOMETRY_MESH_META_BYTE_ALIGNMENT),
                Some("GeometryMeshMetaData".to_string()),
            ),
            geometry_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("GeometryMeshMetaData"),
                    MESH_META_INITIAL_CAPACITY * GEOMETRY_MESH_META_BYTE_ALIGNMENT,
                    BufferUsage::new().with_copy_dst().with_uniform(),
                )
                .into(),
            )?,
            geometry_dirty: true,
            material_buffers: DynamicUniformBuffer::new(
                MESH_META_INITIAL_CAPACITY,
                MATERIAL_MESH_META_BYTE_SIZE,
                Some(MATERIAL_MESH_META_BYTE_ALIGNMENT),
                Some("MaterialMeshMetaData".to_string()),
            ),
            material_gpu_buffer: gpu.create_buffer(
                &BufferDescriptor::new(
                    Some("MaterialMeshMetaData"),
                    MESH_META_INITIAL_CAPACITY * MATERIAL_MESH_META_BYTE_ALIGNMENT,
                    BufferUsage::new().with_copy_dst().with_uniform(),
                )
                .into(),
            )?,
            material_dirty: true,
        })
    }
    pub fn insert(
        &mut self,
        key: MeshKey,
        mesh: &Mesh,
        buffer_info: MeshBufferInfo,
        materials: &Materials,
        transforms: &Transforms,
        morphs: &Morphs,
        skins: &Skins,
    ) -> Result<()> {
        let transform_key = mesh.transform_key;
        let geometry_morph_key = mesh.geometry_morph_key;
        let material_morph_key = mesh.material_morph_key;
        let skin_key = mesh.skin_key;
        let material_key = mesh.material_key;

        let meta_data = GeometryMeshMeta {
            mesh_key: key,
            material_key,
            transform_key,
            geometry_morph_key,
            skin_key,
            materials,
            transforms,
            morphs,
            skins,
        }
        .to_bytes()?;
        self.geometry_buffers.update(key, &meta_data);
        self.geometry_dirty = true;

        // TODO
        // should be basically Vec<MeshBufferVertexAttributeInfo>
        let meta_data = MaterialMeshMeta {
            mesh_key: key,
            material_morph_key,
            material_key,
            materials,
            morphs,
        }
        .to_bytes()?;
        self.material_buffers.update(key, &meta_data);
        self.material_dirty = true;

        Ok(())
    }

    pub fn geometry_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.geometry_gpu_buffer
    }
    pub fn geometry_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.geometry_buffers
            .offset(key)
            .ok_or(AwsmMeshError::MetaNotFound(key))
    }

    pub fn material_gpu_buffer(&self) -> &web_sys::GpuBuffer {
        &self.material_gpu_buffer
    }
    pub fn material_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.material_buffers
            .offset(key)
            .ok_or(AwsmMeshError::MetaNotFound(key))
    }

    pub fn remove(&mut self, mesh_key: MeshKey) {
        if self.geometry_buffers.remove(mesh_key) {
            self.geometry_dirty = true;
        }

        if self.material_buffers.remove(mesh_key) {
            self.material_dirty = true;
        }
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.geometry_dirty {
            if let Some(new_size) = self.geometry_buffers.take_gpu_needs_resize() {
                self.geometry_gpu_buffer = gpu.create_buffer(
                    &BufferDescriptor::new(
                        Some("GeometryMeshMetaData"),
                        new_size,
                        BufferUsage::new().with_copy_dst().with_uniform(),
                    )
                    .into(),
                )?;
                bind_groups.mark_create(BindGroupCreate::GeometryMeshMetaResize);
            }
            gpu.write_buffer(
                &self.geometry_gpu_buffer,
                None,
                self.geometry_buffers.raw_slice(),
                None,
                None,
            )?;

            self.geometry_dirty = false;
        }

        if self.material_dirty {
            if let Some(new_size) = self.material_buffers.take_gpu_needs_resize() {
                self.material_gpu_buffer = gpu.create_buffer(
                    &BufferDescriptor::new(
                        Some("MaterialMeshMetaData"),
                        new_size,
                        BufferUsage::new().with_copy_dst().with_uniform(),
                    )
                    .into(),
                )?;
                bind_groups.mark_create(BindGroupCreate::MaterialMeshMetaResize);
            }
            gpu.write_buffer(
                &self.material_gpu_buffer,
                None,
                self.material_buffers.raw_slice(),
                None,
                None,
            )?;

            self.material_dirty = false;
        }
        Ok(())
    }
}

impl Drop for MeshMeta {
    fn drop(&mut self) {
        self.geometry_gpu_buffer.destroy();
        self.material_gpu_buffer.destroy();
    }
}

// See meta.wgsl for the corresponding struct
pub struct GeometryMeshMeta<'a> {
    pub mesh_key: MeshKey,
    pub transform_key: TransformKey,
    pub material_key: MaterialKey,
    pub geometry_morph_key: Option<GeometryMorphKey>,
    pub skin_key: Option<SkinKey>,
    pub materials: &'a Materials,
    pub transforms: &'a Transforms,
    pub morphs: &'a Morphs,
    pub skins: &'a Skins,
}

impl<'a> GeometryMeshMeta<'a> {
    pub fn to_bytes(
        self,
    ) -> std::result::Result<[u8; GEOMETRY_MESH_META_BYTE_SIZE], AwsmMeshError> {
        let Self {
            mesh_key,
            transform_key,
            material_key,
            geometry_morph_key,
            skin_key,
            materials,
            transforms,
            morphs,
            skins,
        } = self;

        let mut result = [0u8; GEOMETRY_MESH_META_BYTE_SIZE];
        let mut offset = 0;

        let mut push_u32 = |value: u32| {
            result[offset..offset + 4].copy_from_slice(&value.to_le_bytes());

            offset += 4;
        };

        fn bool_as_u32(value: bool) -> u32 {
            if value {
                1
            } else {
                0
            }
        }

        let mesh_key_u64 = mesh_key.data().as_ffi();
        let (mesh_key_u32_high, mesh_key_u32_low) = (
            (mesh_key_u64 >> 32) as u32,
            (mesh_key_u64 & 0xFFFFFFFF) as u32,
        );

        // Mesh Key (8 bytes)
        push_u32(mesh_key_u32_high);
        push_u32(mesh_key_u32_low);

        // Morph (12 bytes)
        if let Some(morph_key) = geometry_morph_key {
            let info = morphs.geometry.get_info(morph_key)?;
            push_u32(info.targets_len as u32);
            push_u32(morphs.geometry.weights_buffer_offset(morph_key)? as u32);
            push_u32(morphs.geometry.values_buffer_offset(morph_key)? as u32);
        } else {
            push_u32(0);
            push_u32(0);
            push_u32(0);
        }

        // Skin (12 bytes)
        if let Some(skin_key) = skin_key {
            push_u32(skins.sets_len(skin_key)? as u32);
            push_u32(skins.joint_matrices_offset(skin_key)? as u32);
            push_u32(skins.joint_index_weights_offset(skin_key)? as u32);
        } else {
            push_u32(0);
            push_u32(0);
            push_u32(0);
        }

        // Transform (4 bytes)
        push_u32(transforms.buffer_offset(transform_key)? as u32);

        // Material (4 bytes)
        push_u32(materials.buffer_offset(material_key)? as u32);

        Ok(result)
    }
}

// See meta.wgsl for the corresponding struct
pub struct MaterialMeshMeta<'a> {
    pub mesh_key: MeshKey,
    pub material_key: MaterialKey,
    pub material_morph_key: Option<MaterialMorphKey>,
    pub materials: &'a Materials,
    pub morphs: &'a Morphs,
}

impl<'a> MaterialMeshMeta<'a> {
    pub fn to_bytes(
        self,
    ) -> std::result::Result<[u8; MATERIAL_MESH_META_BYTE_SIZE], AwsmMeshError> {
        let Self {
            mesh_key,
            material_key,
            material_morph_key,
            materials,
            morphs,
        } = self;

        let mut result = [0u8; MATERIAL_MESH_META_BYTE_SIZE];
        let mut offset = 0;

        let mut push_u32 = |value: u32| {
            result[offset..offset + 4].copy_from_slice(&value.to_le_bytes());

            offset += 4;
        };

        fn bool_as_u32(value: bool) -> u32 {
            if value {
                1
            } else {
                0
            }
        }

        let mesh_key_u64 = mesh_key.data().as_ffi();
        let (mesh_key_u32_high, mesh_key_u32_low) = (
            (mesh_key_u64 >> 32) as u32,
            (mesh_key_u64 & 0xFFFFFFFF) as u32,
        );

        // Mesh Key (8 bytes)
        push_u32(mesh_key_u32_high);
        push_u32(mesh_key_u32_low);

        // Morph (20 bytes)
        if let Some(morph_key) = material_morph_key {
            let info = morphs.material.get_info(morph_key)?;
            push_u32(info.targets_len as u32);
            push_u32(morphs.material.weights_buffer_offset(morph_key)? as u32);
            push_u32(morphs.material.values_buffer_offset(morph_key)? as u32);
        } else {
            push_u32(0);
            push_u32(0);
            push_u32(0);
        }
        if let Some(morph_key) = material_morph_key {
            let info = morphs.material.get_info(morph_key)?;
            let mut bitmask = 0;
            if info.attributes.normal {
                bitmask |= MATERIAL_MESH_META_MORPH_MATERIAL_BITMASK_NORMAL;
            }
            if info.attributes.tangent {
                bitmask |= MATERIAL_MESH_META_MORPH_MATERIAL_BITMASK_TANGENT;
            }
            push_u32(info.targets_len as u32);
            push_u32(bitmask);
        } else {
            push_u32(0);
            push_u32(0);
        }

        // Material (4 bytes)
        push_u32(materials.buffer_offset(material_key)? as u32);

        Ok(result)
    }
}
