use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::{bind_groups::AwsmBindGroupError, textures::SamplerKey};

pub type Result<T> = std::result::Result<T, AwsmPostProcessError>;

#[derive(Error, Debug)]
pub enum AwsmPostProcessError {
    #[error("[post process] Error creating buffer: {0:?}")]
    CreateBuffer(AwsmCoreError),

    #[error("[post process] Error writing buffer: {0:?}")]
    WriteBuffer(AwsmBindGroupError),

    #[error("[post process] Render texture create view: {0}")]
    RenderTextureView(String),

    #[error("[post-process] missing post process sampler {0:?}")]
    MissingPostProcessSampler(SamplerKey),

    #[error("[post-process] {0:?}")]
    CoreError(#[from] AwsmCoreError),
}
