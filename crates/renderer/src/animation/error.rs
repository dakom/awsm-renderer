use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AwsmAnimationError>;

#[derive(Error, Debug)]
pub enum AwsmAnimationError {
    #[error("[transform] {0:?}")]
    Core(#[from] AwsmCoreError),
}