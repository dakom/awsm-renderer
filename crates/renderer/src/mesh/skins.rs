use std::{collections::HashSet, sync::LazyLock};

use awsm_renderer_core::{buffers::{BufferDescriptor, BufferUsage}, error::AwsmCoreError, renderer::AwsmRendererWebGpu};
use glam::Mat4;
use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};
use thiserror::Error;

use crate::{
    bind_groups::{AwsmBindGroupError, BindGroupCreate, BindGroups},
    buffer::dynamic_storage::DynamicStorageBuffer,
    transforms::{TransformKey, Transforms},
    AwsmRenderer, AwsmRendererLogging,
};

impl AwsmRenderer {
    pub fn update_skins(&mut self) {
        let dirty_skin_joints = self.transforms.take_dirty_skin_joints();
        if !dirty_skin_joints.is_empty() {
            self.meshes.skins.update(dirty_skin_joints, &self.transforms);
            self.meshes.skins.gpu_dirty = true;
        }
    }
}

pub struct Skins {
    skeleton_transforms: DenseSlotMap<SkinKey, Vec<TransformKey>>,
    // may be None, in which case its virtually an identity matrix
    inverse_bind_matrices: SecondaryMap<TransformKey, Mat4>,
    skin_matrices: DynamicStorageBuffer<SkinKey>,
    gpu_dirty: bool,
    pub(crate) gpu_buffer: web_sys::GpuBuffer,
}

static BUFFER_USAGE: LazyLock<BufferUsage> = LazyLock::new(|| BufferUsage::new().with_storage().with_copy_dst());
impl Skins {
    pub const SKIN_MATRICES_INITIAL_SIZE: usize = 16 * 4 * 32; // 32 elements is a good starting point

    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let gpu_buffer = gpu.create_buffer(&BufferDescriptor::new(
            Some("Skins"),
            Self::SKIN_MATRICES_INITIAL_SIZE,
            *BUFFER_USAGE,
        ).into())?;

        Ok(Self {
            skeleton_transforms: DenseSlotMap::with_key(),
            inverse_bind_matrices: SecondaryMap::new(),
            skin_matrices: DynamicStorageBuffer::new(
                Self::SKIN_MATRICES_INITIAL_SIZE,
                Some("Skins".to_string()),
            ),
            gpu_dirty: true,
            gpu_buffer
        })
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
                        .update_with_unchecked(skin_key, |matrices| {
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
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Skins GPU write").entered())
            } else {
                None
            };

            if let Some(new_size) = self.skin_matrices.take_gpu_needs_resize() {
                self.gpu_buffer = gpu.create_buffer(&BufferDescriptor::new(
                    Some("Skins"),
                    new_size,
                    *BUFFER_USAGE,
                ).into())?;

                bind_groups.mark_create(BindGroupCreate::SkinJointMatricesResize);
            }

            gpu.write_buffer(&self.gpu_buffer, None, self.skin_matrices.raw_slice(), None, None)?;

            self.gpu_dirty = false;
        }
        Ok(())
    }
}

new_key_type! {
    pub struct SkinKey;
}

pub type Result<T> = std::result::Result<T, AwsmSkinError>;

#[derive(Error, Debug)]
pub enum AwsmSkinError {
    #[error("[skin] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[skin] skin not found: {0:?}")]
    SkinNotFound(SkinKey),

    #[error("[skin] joint transform not found: {joint_transform:?}")]
    JointTransformNotFound { joint_transform: TransformKey },

    #[error("[skin] skin joint matrix mismatch, skin: {skin_key:?}, matrix len: {matrix_len:?} joint_len: {joint_len:?}")]
    SkinJointMatrixMismatch {
        skin_key: SkinKey,
        matrix_len: usize,
        joint_len: usize,
    },

    #[error("[skin] joint already exists but is different: {joint_transform:?}")]
    JointAlreadyExistsButDifferent { joint_transform: TransformKey },

    #[error("[skin] {0:?}")]
    BindGroup(#[from] AwsmBindGroupError),
}
