use super::transforms::TransformKey;
use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AwsmTransformError>;

#[derive(Error, Debug)]
pub enum AwsmTransformError {
    #[error("[transform] local transform does not exist {0:?}")]
    LocalNotFound(TransformKey),

    #[error("[transform] world transform does not exist {0:?}")]
    WorldNotFound(TransformKey),

    #[error("[transform] cannot modify root node")]
    CannotModifyRootNode,

    #[error("[transform] buffer slot missing {0:?}")]
    TransformBufferSlotMissing(TransformKey),

    #[error("[transform] {0:?}")]
    Core(#[from] AwsmCoreError),
}
