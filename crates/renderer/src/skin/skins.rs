use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::Mat4;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::transform::TransformKey;

use super::error::{AwsmSkinError, Result};

pub struct Skins {
    joints: DenseSlotMap<SkinKey, Vec<TransformKey>>,
    // may be None, in which case its virtually an identity matrix
    inverse_bind_matrices: SecondaryMap<TransformKey, Mat4>,
    buffer_bytes: Vec<u8>,
    offsets: SecondaryMap<SkinKey, usize>,
}

impl Skins {
    pub fn new(_gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            joints: DenseSlotMap::with_key(),
            inverse_bind_matrices: SecondaryMap::new(),
            buffer_bytes: Vec::new(),
            offsets: SecondaryMap::new(),
        })
    }

    pub fn insert(
        &mut self,
        joints: Vec<TransformKey>,
        inverse_bind_matrices: Vec<Mat4>,
    ) -> Result<SkinKey> {
        for (index, joint) in joints.iter().enumerate() {
            // check if the inverse bind matrix has diverged
            match (
                self.inverse_bind_matrices.get(*joint),
                inverse_bind_matrices.get(index),
            ) {
                (None, None) => { /* eh, they're the same, let it go */ }
                (Some(a), Some(b)) if a == b => { /* eh, they're the same, let it go */ }
                _ => {
                    return Err(AwsmSkinError::JointAlreadyExistsButDifferent {
                        joint_transform: *joint,
                    });
                }
            }
            self.inverse_bind_matrices.insert(
                *joint,
                inverse_bind_matrices
                    .get(index)
                    .cloned()
                    .unwrap_or(Mat4::IDENTITY),
            );
        }
        let offset = self.buffer_bytes.len();
        self.buffer_bytes
            .extend_from_slice(&vec![0u8; joints.len() * 16 * 4]);
        let skin_key = self.joints.insert(joints);
        self.offsets.insert(skin_key, offset);

        Ok(skin_key)
    }

    pub fn skin_offset(&self, skin_key: SkinKey) -> Result<usize> {
        self.offsets
            .get(skin_key)
            .ok_or(AwsmSkinError::SkinNotFound(skin_key))
            .cloned()
    }

    pub fn update_and_write_gpu(
        &mut self,
        _gpu: &AwsmRendererWebGpu,
        world_matrices: &SecondaryMap<TransformKey, Mat4>,
    ) -> Result<()> {
        // different skins can theoretically share the same joint, so, iterate over them all
        for (skin_key, skin_joints) in self.joints.iter() {
            // safe - we know the skin_key is valid because we just iterated over it
            let offset = *self.offsets.get(skin_key).unwrap();
            for (skin_joint_index, skin_joint) in skin_joints.iter().enumerate() {
                // could cache this for revisited joints, but, it's not a huge deal - might even be faster to redo the math
                let world_matrix = match self.inverse_bind_matrices.get(*skin_joint).cloned() {
                    Some(inverse_bind_matrix) => world_matrices
                        .get(*skin_joint)
                        .map(|m| *m * inverse_bind_matrix)
                        .unwrap(),
                    None => world_matrices.get(*skin_joint).cloned().unwrap(),
                };

                // just overwrite this one matrix
                let bytes = unsafe {
                    std::slice::from_raw_parts(world_matrix.as_ref().as_ptr() as *const u8, 16 * 4)
                };
                let byte_offset = offset + (skin_joint_index * 16 * 4);
                self.buffer_bytes[byte_offset..byte_offset + (16 * 4)].copy_from_slice(bytes);
            }
        }

        // TODO
        //gpu.write_buffer(&joint_matrix_bytes, *offset, joint_matrix_len)?;

        Ok(())
    }
}

new_key_type! {
    pub struct SkinKey;
}
