
use awsm_renderer_core::bind_groups::{BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupLayoutResource, BindGroupResource, BufferBindingLayout, BufferBindingType};
use awsm_renderer_core::buffer::BufferBinding;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, SecondaryMap, SlotMap};

use crate::dynamic_buffer::DynamicBuffer;

use crate::uniforms::bind_group::{BIND_GROUP_MORPH_TARGET_VALUES_BINDING, BIND_GROUP_MORPH_TARGET_WEIGHTS_BINDING};
use super::{MeshKey, error::{Result, AwsmMeshError}};

const MORPH_WEIGHTS_BYTE_SIZE: usize = 32; // 8xf32 is 32 bytes

// The weights are dynamic and updated on a per-mesh basis as frequently as needed
// The values are essentially static, but may be sourced from different (large) buffers
// e.g. they are loaded up front per-gltf file
pub struct Morphs {
    weights_buffer: DynamicBuffer<MeshKey>,
    value_offsets: SecondaryMap<MeshKey, (MorphBufferValuesKey, usize)>,
    value_buffers: SlotMap<MorphBufferValuesKey, web_sys::GpuBuffer>,
    value_bind_groups: SecondaryMap<MorphBufferValuesKey,web_sys::GpuBindGroup>,
    value_bind_group_layouts: SecondaryMap<MorphBufferValuesKey,web_sys::GpuBindGroupLayout>,
}

impl Morphs {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            weights_buffer: DynamicBuffer::new_uniform(MORPH_WEIGHTS_BYTE_SIZE, BIND_GROUP_MORPH_TARGET_WEIGHTS_BINDING, gpu, Some("MorphWeights".to_string()))?,
            value_buffers: SlotMap::with_key(),
            value_bind_groups: SecondaryMap::new(),
            value_bind_group_layouts: SecondaryMap::new(),
            value_offsets: SecondaryMap::new(),
        })
    }

    pub fn insert_mesh(&mut self, key: MeshKey, morph_values_key: MorphBufferValuesKey, value_offset: usize) {
        self.weights_buffer.update(key, &[0; MORPH_WEIGHTS_BYTE_SIZE]);
        self.value_offsets.insert(key, (morph_values_key, value_offset));
    }

    pub fn remove_mesh(&mut self, mesh_key: MeshKey) {
        self.weights_buffer.remove(mesh_key);
    }

    pub fn weights_bind_group(&self) -> &web_sys::GpuBindGroup {
        &self.weights_buffer.bind_group
    }

    pub fn weights_bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        &self.weights_buffer.bind_group_layout
    }

    pub fn weights_buffer_offset(&self, key: MeshKey) -> Result<usize> {
        self.weights_buffer
            .offset(key)
            .ok_or(AwsmMeshError::MorphWeightSlotMissing(key))
    }

    // this does *not* write to the GPU, so it can be called relatively frequently for physics etc.
    pub fn update_morph_weights_with(&mut self, key: MeshKey, len: usize, f: impl FnOnce(&mut [f32])) {
        self.weights_buffer.update_with(key, |slice_u8| {
            let weights_f32 = unsafe {
                std::slice::from_raw_parts_mut(slice_u8.as_ptr() as *mut f32, len)
            };

            f(weights_f32)
        });
    }

    pub fn try_get_morph_value_offset(&self, mesh_key: MeshKey) -> Option<(MorphBufferValuesKey, usize)> {
        self.value_offsets.get(mesh_key).copied()
    }

    pub fn insert_values_buffer(&mut self, gpu: &AwsmRendererWebGpu, buffer: web_sys::GpuBuffer, size: usize) -> Result<MorphBufferValuesKey> {
        let key = self.value_buffers.insert(buffer);

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
                        BufferBinding::new(self.value_buffers.get(key).unwrap())
                            .with_offset(0)
                            .with_size(size),
                    ),
                )],
            )
            .into(),
        );

        self.value_bind_groups.insert(key, bind_group);
        self.value_bind_group_layouts.insert(key, bind_group_layout);

        Ok(key)
    }

    pub fn values_bind_group(&self, morph_values_key: MorphBufferValuesKey) -> &web_sys::GpuBindGroup {
        self.value_bind_groups.get(morph_values_key).unwrap()
    }

    pub fn values_bind_group_layout(&self, morph_values_key: MorphBufferValuesKey) -> &web_sys::GpuBindGroupLayout {
        self.value_bind_group_layouts.get(morph_values_key).unwrap()
    }


    // This *does* write to the gpu, should be called only once per frame
    // just write the entire buffer in one fell swoop
    pub fn write_weights_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        self.weights_buffer.write_to_gpu(gpu)?;

        Ok(())
    }
}

new_key_type! {
    pub struct MorphBufferValuesKey;
}