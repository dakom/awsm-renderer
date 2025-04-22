use std::collections::HashSet;

use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::Mat4;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::{
    buffer::{
        bind_groups::{BindGroupIndex, BindGroups, MeshShapeBindGroupBinding},
        dynamic_buddy::DynamicBuddyBuffer,
    },
    transform::{TransformKey, Transforms},
    AwsmRenderer,
};

use super::error::{AwsmSkinError, Result};

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
    gpu_dirty: bool,
}

impl Default for Skins {
    fn default() -> Self {
        Self::new()
    }
}

impl Skins {
    pub const SKIN_MATRICES_INITIAL_SIZE: usize = 16 * 4 * 32; // 32 elements is a good starting point

    pub fn new() -> Self {
        Self {
            skeleton_transforms: DenseSlotMap::with_key(),
            inverse_bind_matrices: SecondaryMap::new(),
            skin_matrices: DynamicBuddyBuffer::new(
                Self::SKIN_MATRICES_INITIAL_SIZE,
                Some("Skins".to_string()),
            ),
            gpu_dirty: true,
        }
    }

    pub fn insert(
        &mut self,
        skeleton_joint_transforms: Vec<TransformKey>,
        inverse_bind_matrices: Vec<Mat4>,
    ) -> Result<SkinKey> {
        let len = skeleton_joint_transforms.len();
        let mut initial_fill = Vec::with_capacity(len * 16 * 4);

        for (index, joint) in skeleton_joint_transforms.iter().enumerate() {
            // check if the inverse bind matrix has diverged
            match (
                self.inverse_bind_matrices.get(*joint),
                inverse_bind_matrices.get(index),
            ) {
                (None, None) => { /* eh, they're the same, let it go */ }
                (None, Some(_)) => { /* it's probably just a new one, let it go */ }
                (Some(a), Some(b)) if a == b => { /* eh, they're the same, let it go */ }
                _ => {
                    return Err(AwsmSkinError::JointAlreadyExistsButDifferent {
                        joint_transform: *joint,
                    });
                }
            }

            let joint_matrix = inverse_bind_matrices
                .get(index)
                .cloned()
                .unwrap_or(Mat4::IDENTITY);

            //tracing::info!("{}: {:#?}", index, joint_matrix);

            let bytes = unsafe {
                std::slice::from_raw_parts(joint_matrix.as_ref().as_ptr() as *const u8, 16 * 4)
            };
            initial_fill.extend_from_slice(bytes);

            self.inverse_bind_matrices.insert(*joint, joint_matrix);
        }

        let skin_key = self.skeleton_transforms.insert(skeleton_joint_transforms);

        self.skin_matrices.update(skin_key, &initial_fill);

        Ok(skin_key)
    }

    pub fn joint_matrices_offset(&self, skin_key: SkinKey) -> Result<usize> {
        self.skin_matrices
            .offset(skin_key)
            .ok_or(AwsmSkinError::SkinNotFound(skin_key))
    }

    pub fn update(&mut self, dirty_skin_joints: HashSet<TransformKey>, transforms: &Transforms) {
        // different skins can theoretically share the same joint, so, iterate over them all
        for (skin_key, skeleton_joints) in self.skeleton_transforms.iter() {
            for (index, skeleton_joint) in skeleton_joints.iter().enumerate() {
                if dirty_skin_joints.contains(skeleton_joint) {
                    // could cache this for revisited joints, but, it's not a huge deal - might even be faster to redo the math
                    let world_matrix =
                        match self.inverse_bind_matrices.get(*skeleton_joint).cloned() {
                            Some(inverse_bind_matrix) => transforms
                                .get_world(*skeleton_joint)
                                .map(|m| *m * inverse_bind_matrix)
                                .unwrap(),
                            None => transforms.get_world(*skeleton_joint).cloned().unwrap(),
                        };

                    // just overwrite this one matrix
                    let bytes = unsafe {
                        std::slice::from_raw_parts(
                            world_matrix.as_ref().as_ptr() as *const u8,
                            16 * 4,
                        )
                    };

                    self.skin_matrices
                        .update_with_unchecked(skin_key, &mut |matrices| {
                            let start = index * 16 * 4;
                            matrices[start..start + (16 * 4)].copy_from_slice(bytes);
                        });
                }
            }

            //tracing::info!("{:#?}", u8_to_f32_vec(&self.skin_matrices.raw_slice()[self.skin_matrices.offset(skin_key).unwrap()..]).chunks(16).take(2).collect::<Vec<_>>());
        }
    }

    pub fn write_gpu(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.gpu_dirty {
            let bind_group_index =
                BindGroupIndex::MeshShape(MeshShapeBindGroupBinding::SkinJointMatrices);
            if let Some(new_size) = self.skin_matrices.take_gpu_needs_resize() {
                bind_groups.gpu_resize(gpu, bind_group_index, new_size)?;
            }

            bind_groups.gpu_write(
                gpu,
                bind_group_index,
                None,
                self.skin_matrices.raw_slice(),
                None,
                None,
            )?;
            self.gpu_dirty = false;
        }
        Ok(())
    }
}

new_key_type! {
    pub struct SkinKey;
}
