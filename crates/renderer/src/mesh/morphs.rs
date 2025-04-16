
use awsm_renderer_core::bind_groups::{BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupLayoutResource, BindGroupResource, BufferBindingLayout, BufferBindingType};
use awsm_renderer_core::buffer::BufferBinding;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, SlotMap};

use crate::buffers::helpers::slice_zeroes;
use crate::buffers::{dynamic::DynamicBuffer, bind_group::{BIND_GROUP_MORPH_TARGET_VALUES_BINDING, BIND_GROUP_MORPH_TARGET_WEIGHTS_BINDING}};
use super::MeshBufferMorphInfo;
use super::error::{Result, AwsmMeshError};

const MORPH_WEIGHTS_BYTE_SIZE: usize = 32; // 8xf32 is 32 bytes
const MORPH_WEIGHTS_BYTE_ALIGNMENT: usize = 256; // minUniformBufferOffsetAlignment
const MORPH_TARGETS:usize = 2;
const MORPH_VALUES_BYTE_ALIGNMENT: usize = 48 * MORPH_TARGETS; // 4 bytes per float, 3 floats per vec3, 3 vec3's per struct = 36. Nearest padding = 48

// The weights are dynamic and updated on a per-mesh basis as frequently as needed
// The values are essentially static, but may be sourced from different (large) buffers
// e.g. they are loaded up front per-gltf file
pub struct Morphs {
    weights: DynamicBuffer<MorphKey>,
    infos: SlotMap<MorphKey, MorphInfo>,
}

pub struct MorphInfo {
    pub morph_buffer_info: MeshBufferMorphInfo,
    pub bind_group: web_sys::GpuBindGroup,
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
}

impl Morphs {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            weights: DynamicBuffer::new_uniform(MORPH_WEIGHTS_BYTE_SIZE, MORPH_WEIGHTS_BYTE_ALIGNMENT, BIND_GROUP_MORPH_TARGET_WEIGHTS_BINDING, gpu, Some("MorphWeights".to_string()))?,
            infos: SlotMap::with_key()
        })
    }

    pub fn get_info(&self, key: MorphKey) -> Result<&MorphInfo> {
        self.infos.get(key).ok_or_else(|| {
            AwsmMeshError::MorphNotFound(key)
        })
    }

    pub fn insert(&mut self, gpu: &AwsmRendererWebGpu, buffer: &web_sys::GpuBuffer, morph_buffer_info: MeshBufferMorphInfo) -> Result<MorphKey> {

        let layout_entry = BindGroupLayoutEntry::new(
            BIND_GROUP_MORPH_TARGET_VALUES_BINDING,
            BindGroupLayoutResource::Buffer(
                BufferBindingLayout::new()
                    .with_binding_type(BufferBindingType::ReadOnlyStorage)
                    .with_dynamic_offset(true)
            ),
        ).with_visibility_vertex();
                
        let bind_group_layout = gpu.create_bind_group_layout(
            &BindGroupLayoutDescriptor::new(Some("MorphTargetValues"))
                .with_entries(vec![layout_entry])
                .into(),
        )?;

        let bind_group = gpu.create_bind_group(
            &BindGroupDescriptor::new(
                &bind_group_layout,
                Some("MorphTargetValues"),
                vec![BindGroupEntry::new(
                    BIND_GROUP_MORPH_TARGET_VALUES_BINDING,
                    BindGroupResource::Buffer(
                        BufferBinding::new(buffer)
                            .with_offset(0)
                            .with_size(morph_buffer_info.size)
                    ),
                )],
            )
            .into(),
        );

        let key = self.infos.insert(
            MorphInfo {
                morph_buffer_info,
                bind_group,
                bind_group_layout: bind_group_layout.clone(),
            },
        );

        self.weights.update(key, &[0u8;MORPH_WEIGHTS_BYTE_SIZE]);

        Ok(key)
    }

    pub fn remove(&mut self, key: MorphKey) {
        self.weights.remove(key);
        self.infos.remove(key);
    }

    pub fn weights_bind_group(&self) -> &web_sys::GpuBindGroup {
        &self.weights.bind_group
    }

    pub fn weights_bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        &self.weights.bind_group_layout
    }

    pub fn weights_buffer_offset(&self, key: MorphKey) -> Result<usize> {
        self.weights
            .offset(key)
            .ok_or_else(|| AwsmMeshError::MorphNotFound(key))
    }

    // this does *not* write to the GPU, so it can be called relatively frequently for physics etc.
    pub fn update_morph_weights_with(&mut self, key: MorphKey, f: impl FnOnce(&mut [f32])) -> Result<()> {
        let len = self.get_info(key).map(|info| info.morph_buffer_info.targets_len)?;

        self.weights.update_with(key, |slice_u8| {
            let weights_f32 = unsafe {
                std::slice::from_raw_parts_mut(slice_u8.as_ptr() as *mut f32, len)
            };

            f(weights_f32)
        });

        Ok(())
    }

    pub fn values_bind_group(&self, key: MorphKey) -> Result<&web_sys::GpuBindGroup> {
        self.get_info(key).map(|v| &v.bind_group)
    }

    pub fn values_bind_group_layout(&self, key: MorphKey) -> Result<&web_sys::GpuBindGroupLayout> {
        self.get_info(key).map(|v| &v.bind_group_layout)
    }

    pub fn values_buffer_offset(&self, key: MorphKey) -> Result<usize> {
        self.get_info(key).map(|info| info.morph_buffer_info.offset)
    }


    // This *does* write to the gpu, should be called only once per frame
    // just write the entire buffer in one fell swoop
    pub fn write_weights_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        // let keys = self.weights.keys().collect::<Vec<_>>();
        // for key in keys {
        //     self.update_morph_weights_with(key, |weights| {
        //         weights[0] = 0.0;
        //         weights[1] = 1.0;
        //     })?;

        // }
        self.weights.write_to_gpu(gpu)?;

        Ok(())
    }
}

new_key_type! {
    pub struct MorphKey;
}