use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::transform::AwsmTransformError;

use super::MeshKey;

pub type Result<T> = std::result::Result<T, AwsmMeshError>;

#[derive(Error, Debug)]
pub enum AwsmMeshError {
    #[error("[mesh] not found: {0:?}")]
    MeshNotFound(MeshKey),

    #[error("[mesh] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[mesh] {0:?}")]
    Transform(#[from] AwsmTransformError),
}
