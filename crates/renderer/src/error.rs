use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::{camera::AwsmCameraError, mesh::AwsmMeshError, transform::AwsmTransformError};

#[derive(Error, Debug)]
pub enum AwsmError {
    #[error("{0}")]
    Core(#[from] AwsmCoreError),

    #[error("{0}")]
    Camera(#[from] AwsmCameraError),

    #[error("{0}")]
    Mesh(#[from] AwsmMeshError),

    #[error("{0}")]
    Transform(#[from] AwsmTransformError),

    #[cfg(feature = "animation")]
    #[error("{0}")]
    Animation(#[from] crate::animation::AwsmAnimationError),
}

pub type Result<T> = std::result::Result<T, AwsmError>;
