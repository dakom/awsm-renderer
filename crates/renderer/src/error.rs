use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::camera::AwsmCameraError;

#[derive(Error, Debug)]
pub enum AwsmError {
    #[error("{0}")]
    Core(#[from] AwsmCoreError),

    #[error("{0}")]
    Camera(#[from] AwsmCameraError),
}

pub type Result<T> = std::result::Result<T, AwsmError>;
