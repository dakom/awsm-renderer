use std::collections::HashSet;

use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::Mat4;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::{buffers::{bind_group::BIND_GROUP_SKIN_JOINT_MATRICES_BINDING, dynamic::DynamicBufferKind, dynamic_buddy::DynamicBuddyBuffer}, transform::{TransformKey, Transforms}, AwsmRenderer};

use super::error::{AwsmSkinError, Result};

const SKIN_MATRICES_INITIAL_BYTES: usize = 1024; // 32 elements is a good starting point

impl AwsmRenderer {
    pub fn update_skins(&mut self) {
        let dirty_skin_joints = self.transforms.take_dirty_skin_joints();
        if !dirty_skin_joints.is_empty() {
            self.skins.update(dirty_skin_joints, &self.transforms);
            self.skins.gpu_dirty = true;
        }
    }
}

pub struct Skins {
    skeleton_transforms: DenseSlotMap<SkinKey, Vec<TransformKey>>,
    // may be None, in which case its virtually an identity matrix
    inverse_bind_matrices: SecondaryMap<TransformKey, Mat4>,
    skin_matrices: DynamicBuddyBuffer<SkinKey>,
    gpu_dirty: bool
}

impl Skins {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            skeleton_transforms: DenseSlotMap::with_key(),
            inverse_bind_matrices: SecondaryMap::new(),
            skin_matrices: DynamicBuddyBuffer::new(
                SKIN_MATRICES_INITIAL_BYTES,
                DynamicBufferKind::new_uniform(BIND_GROUP_SKIN_JOINT_MATRICES_BINDING),
                gpu,
                Some("Skins".to_string()),
            )?,
            gpu_dirty: true,
        })
    }

    pub fn insert(
        &mut self,
        skeleton_joint_transforms: Vec<TransformKey>,
        inverse_bind_matrices: Vec<Mat4>,
    ) -> Result<SkinKey> {
        for (index, joint) in skeleton_joint_transforms.iter().enumerate() {
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

        let len = skeleton_joint_transforms.len();

        let skin_key = self.skeleton_transforms.insert(skeleton_joint_transforms);

        self.skin_matrices.update(skin_key, &vec![0;16 * 4 * len]);

        Ok(skin_key)
    }

    pub fn joint_matrices_offset(&self, skin_key: SkinKey) -> Result<usize> {
        self.skin_matrices.offset(skin_key).ok_or(AwsmSkinError::SkinNotFound(skin_key))
    }

    pub fn joint_matrices_bind_group(&self) -> &web_sys::GpuBindGroup {
        self.skin_matrices.bind_group.as_ref().unwrap()
    }

    pub fn joint_matrices_bind_group_layout(&self) -> &web_sys::GpuBindGroupLayout {
        self.skin_matrices.bind_group_layout.as_ref().unwrap()
    }

    pub fn update(
        &mut self,
        dirty_skin_joints: HashSet<TransformKey>,
        transforms: &Transforms,
    ) {
        // different skins can theoretically share the same joint, so, iterate over them all
        for (skin_key, skeleton_joints) in self.skeleton_transforms.iter() {
            for (index, skeleton_joint) in skeleton_joints.iter().enumerate() {
                if dirty_skin_joints.contains(skeleton_joint) {
                    // could cache this for revisited joints, but, it's not a huge deal - might even be faster to redo the math
                    let world_matrix = match self.inverse_bind_matrices.get(*skeleton_joint).cloned() {
                        Some(inverse_bind_matrix) => transforms.get_world(*skeleton_joint)
                            .map(|m| *m * inverse_bind_matrix)
                            .unwrap(),
                        None => transforms.get_world(*skeleton_joint).cloned().unwrap(),
                    };

                    // just overwrite this one matrix
                    self.skin_matrices.update_with_unchecked(skin_key, &mut |values_u8| {
                        let offset = 16 * 4 * index;
                        let bytes = unsafe {
                            std::slice::from_raw_parts(world_matrix.as_ref().as_ptr() as *const u8, 16 * 4)
                        };

                        values_u8[offset..offset + (16 * 4)].copy_from_slice(bytes);
                    });
                }
            }
        }
    }

    pub fn write_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        if self.gpu_dirty {
            self.skin_matrices.write_to_gpu(gpu)?;
            self.gpu_dirty = false;
        }
        Ok(())
    }
}

new_key_type! {
    pub struct SkinKey;
}
