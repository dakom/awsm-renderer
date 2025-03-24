use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AwsmError {
    #[error("{0}")]
    Core(#[from] AwsmCoreError),

}

pub type Result<T> = std::result::Result<T, AwsmError>;
