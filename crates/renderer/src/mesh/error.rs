use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::{buffer::bind_groups::AwsmBindGroupError, transform::AwsmTransformError};

use super::{morphs::MorphKey, MeshKey};

pub type Result<T> = std::result::Result<T, AwsmMeshError>;

#[derive(Error, Debug)]
pub enum AwsmMeshError {
    #[error("[mesh] not found: {0:?}")]
    MeshNotFound(MeshKey),

    #[error("[mesh] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[mesh] {0:?}")]
    Transform(#[from] AwsmTransformError),

    #[error("[mesh] morph not found: {0:?}")]
    MorphNotFound(MorphKey),

    #[error("[mesh] morph must have same number of weights as targets: {weights} weights != {targets} targets")]
    MorphWeightsTargetsMismatch {
        weights: usize,
        targets: usize,
    },

    #[error("[mesh] {0:?}")]
    BindGroup(#[from] AwsmBindGroupError),
}
