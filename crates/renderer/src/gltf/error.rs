use awsm_renderer_core::{error::AwsmCoreError, pipeline::primitive::IndexFormat};
use gltf::Semantic;
use thiserror::Error;

use crate::{
    animation::AwsmAnimationError, bind_groups::AwsmBindGroupError, materials::AwsmMaterialError,
    mesh::AwsmMeshError, shaders::AwsmShaderError, skin::AwsmSkinError,
    transform::AwsmTransformError,
};

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

    #[error("[gltf] Unsupported index format: {0:?}")]
    UnsupportedIndexFormat(IndexFormat),

    #[error("[gltf] Unsupported integer conversion: {0:?}")]
    UnsupportedIntConversion(#[from] std::num::TryFromIntError),

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

    #[error("[gltf] mesh primitive shader: {0:?}")]
    MeshPrimitiveShader(AwsmCoreError),

    #[error("[gltf] mesh primitive render pipeline: {0:?}")]
    MeshPrimitiveRenderPipeline(AwsmCoreError),

    #[error("[gltf] {0:?}")]
    Animation(#[from] AwsmAnimationError),

    #[error("[gltf] {0:?}")]
    Skin(#[from] AwsmSkinError),

    #[error("[gltf] morph animation exists but no morph target found")]
    MissingMorphForAnimation,

    #[error("[gltf] missing animation sampler. animation_index: {animation_index}, channel_index: {channel_index}, sampler_index: {sampler_index}")]
    MissingAnimationSampler {
        animation_index: usize,
        channel_index: usize,
        sampler_index: usize,
    },

    #[error("[gltf] invalid skin joint count. joints: {joint_count}, inverse_bind_matrices: {matrix_count}")]
    InvalidSkinInverseBindMatrixCount {
        matrix_count: usize,
        joint_count: usize,
    },

    #[error("[gltf] skin joint transform not found: {0}")]
    SkinJointTransformNotFound(usize),

    #[error("[gltf] shader key has different joint and weight count: ({weight_sets} weight sets and {joint_sets} joint sets)")]
    ShaderKeyDifferentJointsWeights { weight_sets: u32, joint_sets: u32 },

    #[error("[gltf] could not get shader location for semantic: {0:?}")]
    ShaderLocationNoSemantic(Semantic),

    #[error("[gltf] {0:?}")]
    Shader(#[from] AwsmShaderError),

    #[error("[gltf] could not convert transform to winding order: {0:?}")]
    TransformToWindingOrder(AwsmTransformError),

    #[error("[gltf] instancing extension: {0:?}")]
    ExtInstancing(anyhow::Error),

    #[error("[gltf] create texture: {0:?}")]
    CreateTexture(AwsmCoreError),

    #[error("[gltf] missing texture index in doc: {0}")]
    MissingTextureDocIndex(usize),

    #[error("[gltf] missing texture index: {0}")]
    MissingTextureIndex(usize),

    #[error("[gltf] unable to create texture view: {0}")]
    CreateTextureView(String),

    #[error("[gltf] unable to create material bind group: {0:?}")]
    MaterialBindGroup(AwsmBindGroupError),

    #[error("[gltf] unable to create material bind group layout: {0:?}")]
    MaterialBindGroupLayout(AwsmBindGroupError),

    #[error("[gltf] missing material bind group layout: {0:?}")]
    MaterialMissingBindGroupLayout(AwsmBindGroupError),

    #[error("[gltf] material: {0:?}")]
    Material(#[from] AwsmMaterialError),

    #[error("[gltf] unable to construct normals: {0}")]
    ConstructNormals(String),
}

pub type Result<T> = std::result::Result<T, AwsmGltfError>;
