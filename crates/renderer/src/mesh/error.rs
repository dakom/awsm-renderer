use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::{
    bind_groups::AwsmBindGroupError,
    materials::{AwsmMaterialError, MaterialKey},
    mesh::{
        morphs::{GeometryMorphKey, MaterialMorphKey},
        skins::AwsmSkinError,
        MeshBufferInfoKey,
    },
    transforms::AwsmTransformError,
};

use super::MeshKey;

pub type Result<T> = std::result::Result<T, AwsmMeshError>;

#[derive(Error, Debug)]
pub enum AwsmMeshError {
    #[error("[mesh] not found: {0:?}")]
    MeshNotFound(MeshKey),

    #[error("[mesh] visibility buffer not found: {0:?}")]
    VisibilityBufferNotFound(MeshKey),

    #[error("[mesh] attribute buffer not found: {0:?}")]
    AttributeBufferNotFound(MeshKey),

    #[error("[mesh] metadata not found: {0:?}")]
    MetaNotFound(MeshKey),

    #[error("[mesh] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[mesh] {0:?}")]
    Transform(#[from] AwsmTransformError),

    #[error("[mesh] {0:?}")]
    Material(#[from] AwsmMaterialError),

    #[error("[mesh] {0:?}")]
    Skin(#[from] AwsmSkinError),

    #[error("[mesh] morph not found: {0}")]
    MorphNotFound(String),

    #[error("[mesh] morph must have same number of weights as targets: {weights} weights != {targets} targets")]
    MorphWeightsTargetsMismatch { weights: usize, targets: usize },

    #[error("[mesh] {0:?}")]
    BindGroup(#[from] AwsmBindGroupError),

    #[error("[mesh] buffer info not found: {0:?}")]
    BufferInfoNotFound(MeshBufferInfoKey),
}
