use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, SlotMap};

use super::error::{AwsmMeshError, Result};
use super::MeshBufferMorphInfo;
use crate::buffers::dynamic_buddy::DynamicBuddyBuffer;
use crate::buffers::{
    bind_group::{BIND_GROUP_MORPH_TARGET_VALUES_BINDING, BIND_GROUP_MORPH_TARGET_WEIGHTS_BINDING},
    dynamic_fixed::DynamicFixedBuffer,
};

const MORPH_WEIGHTS_BYTE_SIZE: usize = 32; // 8xf32 is 32 bytes
const MORPH_WEIGHTS_BYTE_ALIGNMENT: usize = 256; // minUniformBufferOffsetAlignment
const MORPH_VALUES_INITIAL_SIZE: usize = 4096; // 4kB is a good starting point

// The weights are dynamic and updated on a per-mesh basis as frequently as needed
// The values are essentially static, but may be sourced from different (large) buffers
// e.g. they are loaded up front per-gltf file
pub struct Morphs {
    weights: DynamicFixedBuffer<MorphKey>,
    values: DynamicBuddyBuffer<MorphKey>,
    weights_dirty: bool,
    values_dirty: bool,
    infos: SlotMap<MorphKey, MeshBufferMorphInfo>,
}

impl Morphs {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            weights: DynamicFixedBuffer::new_uniform(
                MORPH_WEIGHTS_BYTE_SIZE,
                MORPH_WEIGHTS_BYTE_ALIGNMENT,
                BIND_GROUP_MORPH_TARGET_WEIGHTS_BINDING,
                gpu,
                Some("MorphWeights".to_string()),
            )?,
            values: DynamicBuddyBuffer::new_storage(
                MORPH_VALUES_INITIAL_SIZE,
                BIND_GROUP_MORPH_TARGET_VALUES_BINDING,
                gpu,
                Some("MorphTargetValues".to_string()),
            )?,
            weights_dirty: true,
            values_dirty: true,
            infos: SlotMap::with_key(),
        })
    }

    pub fn get_info(&self, key: MorphKey) -> Result<&MeshBufferMorphInfo> {
        self.infos.get(key).ok_or(AwsmMeshError::MorphNotFound(key))
    }

    pub fn insert(
        &mut self,
        morph_buffer_info: MeshBufferMorphInfo,
        bytes: &[u8],
    ) -> Result<MorphKey> {
        let key = self.infos.insert(morph_buffer_info.clone());
        self.weights.update(key, &[0u8; MORPH_WEIGHTS_BYTE_SIZE]);
        self.values.update(key, bytes);

        self.weights_dirty = true;
        self.values_dirty = true;

        Ok(key)
    }

    pub fn remove(&mut self, key: MorphKey) {
        self.weights.remove(key);
        self.values.remove(key);
        self.infos.remove(key);

        self.weights_dirty = true;
        self.values_dirty = true;
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
            .ok_or(AwsmMeshError::MorphNotFound(key))
    }

    pub fn values_bind_group(&self) -> &web_sys::GpuBindGroup {
        &self.values.bind_group
    }
    pub fn values_bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        &self.values.bind_group_layout
    }
    pub fn values_buffer_offset(&self, key: MorphKey) -> Result<usize> {
        self.values
            .offset(key)
            .ok_or(AwsmMeshError::MorphNotFound(key))
    }

    // this does *not* write to the GPU, so it can be called relatively frequently for physics etc.
    pub fn update_morph_weights_with(
        &mut self,
        key: MorphKey,
        f: impl FnOnce(&mut [f32]),
    ) -> Result<()> {
        let len = self.get_info(key).map(|info| info.targets_len)?;

        self.weights.update_with(key, |slice_u8| {
            let weights_f32 =
                unsafe { std::slice::from_raw_parts_mut(slice_u8.as_ptr() as *mut f32, len) };

            f(weights_f32)
        });

        self.weights_dirty = true;

        Ok(())
    }

    // This *does* write to the gpu, should be called only once per frame
    // just write the entire buffer in one fell swoop
    pub fn write_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        if self.weights_dirty {
            self.weights.write_to_gpu(gpu)?;
            self.weights_dirty = false;
        }
        if self.values_dirty {
            self.values.write_to_gpu(gpu)?;
            self.values_dirty = false;
        }

        Ok(())
    }
}

new_key_type! {
    pub struct MorphKey;
}
