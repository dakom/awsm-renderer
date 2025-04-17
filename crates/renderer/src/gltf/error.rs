use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::{animation::AwsmAnimationError, mesh::AwsmMeshError};

#[derive(Error, Debug)]
pub enum AwsmGltfError {
    #[error("[gltf] TODO: {0}")]
    Todo(String),

    #[error("[gltf] Error loading file")]
    Load,

    #[error("[gltf] No scene at index {0}")]
    InvalidScene(usize),

    #[error("[gltf] No default scene (or explicitly provided scene)")]
    NoDefaultScene,

    #[error("[gltf] Unable to create buffer from accessor: {0}")]
    BufferAccessor(String),

    #[error("[gltf] Unable to create buffer: {0:?}")]
    BufferCreate(AwsmCoreError),

    #[error("[gltf] Unable to write buffer: {0:?}")]
    BufferWrite(AwsmCoreError),

    #[error("[gltf] Unable to create bind group layout: {0:?}")]
    BindGroupLayout(AwsmCoreError),

    #[error("[gltf] Unsupported primitive mode: {0:?}")]
    UnsupportedPrimitiveMode(gltf::mesh::Mode),

    #[error("[gltf] missing positions attribute: {0:?}")]
    MissingPositionAttribute(gltf::mesh::Semantic),

    #[error("[gltf] Unsupported index data type: {0:?}")]
    UnsupportedIndexDataType(gltf::accessor::DataType),

    #[error("[gltf] Invalid sparse index size: {0:?}")]
    InvalidSparseIndexSize(gltf::accessor::DataType),

    #[error("[gltf] unsupported morph semantic: {0:?}")]
    UnsupportedMorphSemantic(gltf::mesh::Semantic),

    #[error("[gltf] morph storage key missing")] 
    MorphStorageKeyMissing,

    #[error("[gltf] invalid morph buffer size: {0}")] 
    InvalidMorphBufferSize(String),

    #[error("[gltf] {0:?}")]
    Mesh(#[from] AwsmMeshError),

    #[error("[gltf] {0:?}")]
    Animation(#[from] AwsmAnimationError),

    #[error("[gltf] morph animation exists but no morph target found")]
    MissingMorphForAnimation,
}

pub type Result<T> = std::result::Result<T, AwsmGltfError>;
