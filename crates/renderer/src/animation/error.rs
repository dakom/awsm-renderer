use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::{mesh::AwsmMeshError, transform::AwsmTransformError};

use super::AnimationKey;

pub type Result<T> = std::result::Result<T, AwsmAnimationError>;

#[derive(Error, Debug)]
pub enum AwsmAnimationError {
    #[error("[animation] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[animation] {0:?}")]
    Transform(#[from] AwsmTransformError),

    #[error("[animation] {0:?}")]
    Mesh(#[from] AwsmMeshError),

    #[error("[animation] {0}")]
    WrongKind(String),

    #[error("[animation] missing animation key {0:?}")]
    MissingKey(AnimationKey),
}