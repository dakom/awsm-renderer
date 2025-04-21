use crate::skin::AwsmSkinError;

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

    #[error("[transform] cannot get parent of root node")]
    CannotGetParentOfRootNode,

    #[error("[transform] cannot get parent for {0:?}")]
    CannotGetParent(TransformKey),

    #[error("[transform] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[transform] {0:?}")]
    Skin(#[from] AwsmSkinError),
}
