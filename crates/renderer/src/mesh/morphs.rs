use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, SlotMap};

use super::error::{AwsmMeshError, Result};
use super::MeshBufferMorphInfo;
use crate::bind_groups::{
    uniform_storage::MeshShapeBindGroupBinding, uniform_storage::UniformStorageBindGroupIndex,
    BindGroups,
};
use crate::buffer::dynamic_storage::DynamicStorageBuffer;
use crate::AwsmRendererLogging;

// The weights are dynamic and updated on a per-mesh basis as frequently as needed
// The values are essentially static, but may be sourced from different (large) buffers
// e.g. they are loaded up front per-gltf file
pub struct Morphs {
    weights: DynamicStorageBuffer<MorphKey>,
    values: DynamicStorageBuffer<MorphKey>,
    weights_dirty: bool,
    values_dirty: bool,
    infos: SlotMap<MorphKey, MeshBufferMorphInfo>,
}

impl Default for Morphs {
    fn default() -> Self {
        Self::new()
    }
}

impl Morphs {
    pub const WEIGHTS_INITIAL_SIZE: usize = 4096; // 4kB is a good starting point
    pub const VALUES_INITIAL_SIZE: usize = 4096; // 4kB is a good starting point

    pub fn new() -> Self {
        Self {
            weights: DynamicStorageBuffer::new(
                Self::WEIGHTS_INITIAL_SIZE,
                Some("MorphWeights".to_string()),
            ),
            values: DynamicStorageBuffer::new(
                Self::VALUES_INITIAL_SIZE,
                Some("MorphValues".to_string()),
            ),
            weights_dirty: true,
            values_dirty: true,
            infos: SlotMap::with_key(),
        }
    }

    pub fn get_info(&self, key: MorphKey) -> Result<&MeshBufferMorphInfo> {
        self.infos.get(key).ok_or(AwsmMeshError::MorphNotFound(key))
    }

    pub fn insert(
        &mut self,
        morph_buffer_info: MeshBufferMorphInfo,
        weights: &[f32],
        value_bytes: &[u8],
    ) -> Result<MorphKey> {
        if weights.len() != morph_buffer_info.targets_len {
            return Err(AwsmMeshError::MorphWeightsTargetsMismatch {
                weights: weights.len(),
                targets: morph_buffer_info.targets_len,
            });
        }

        let mut weights_and_count: Vec<f32> = Vec::with_capacity(weights.len() + 1);
        weights_and_count.push(weights.len() as f32);
        weights_and_count.extend_from_slice(weights);
        let key = self.infos.insert(morph_buffer_info.clone());
        let weights_u8 = unsafe {
            std::slice::from_raw_parts(
                weights_and_count.as_ptr() as *const u8,
                4 + (weights.len() * 4),
            )
        };
        self.weights.update(key, weights_u8);
        self.values.update(key, value_bytes);

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

    pub fn weights_buffer_offset(&self, key: MorphKey) -> Result<usize> {
        self.weights
            .offset(key)
            .ok_or(AwsmMeshError::MorphNotFound(key))
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

        self.weights.update_with_unchecked(key, |slice_u8| {
            let weights_f32 =
                unsafe { std::slice::from_raw_parts_mut(slice_u8.as_ptr() as *mut f32, len + 1) };

            // The first value is the number of targets
            let weights_f32 = &mut weights_f32[1..];

            f(weights_f32)
        });

        self.weights_dirty = true;

        Ok(())
    }

    // This *does* write to the gpu, should be called only once per frame
    // just write the entire buffer in one fell swoop
    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.weights_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Morph Weights GPU write").entered())
            } else {
                None
            };
            let bind_group_index = UniformStorageBindGroupIndex::MeshShape(
                MeshShapeBindGroupBinding::MorphTargetWeights,
            );
            if let Some(new_size) = self.weights.take_gpu_needs_resize() {
                bind_groups
                    .uniform_storages
                    .gpu_resize(gpu, bind_group_index, new_size)?;
            }
            bind_groups.uniform_storages.gpu_write(
                gpu,
                bind_group_index,
                None,
                self.weights.raw_slice(),
                None,
                None,
            )?;
            self.weights_dirty = false;
        }
        if self.values_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Morph Values GPU write").entered())
            } else {
                None
            };
            let bind_group_index = UniformStorageBindGroupIndex::MeshShape(
                MeshShapeBindGroupBinding::MorphTargetValues,
            );
            if let Some(new_size) = self.values.take_gpu_needs_resize() {
                bind_groups
                    .uniform_storages
                    .gpu_resize(gpu, bind_group_index, new_size)?;
            }
            bind_groups.uniform_storages.gpu_write(
                gpu,
                bind_group_index,
                None,
                self.values.raw_slice(),
                None,
                None,
            )?;
            self.values_dirty = false;
        }

        Ok(())
    }
}

new_key_type! {
    pub struct MorphKey;
}
