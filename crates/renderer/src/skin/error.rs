use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::transform::TransformKey;

use super::skins::SkinKey;

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
}
