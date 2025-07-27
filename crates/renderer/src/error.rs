use awsm_renderer_core::error::AwsmCoreError;
use thiserror::Error;

use crate::{
    bind_groups::AwsmBindGroupError, camera::AwsmCameraError, instances::AwsmInstanceError, lights::AwsmLightError, materials::AwsmMaterialError, mesh::AwsmMeshError, pipeline::AwsmPipelineError, render::post_process::error::AwsmPostProcessError, shaders::AwsmShaderError, skin::AwsmSkinError, transform::AwsmTransformError
};

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

    #[error("{0}")]
    Skin(#[from] AwsmSkinError),

    #[error("{0}")]
    BindGroup(#[from] AwsmBindGroupError),

    #[error("{0}")]
    Shader(#[from] AwsmShaderError),

    #[error("{0}")]
    Instance(#[from] AwsmInstanceError),

    #[error("{0}")]
    Material(#[from] AwsmMaterialError),

    #[error("{0}")]
    Pipeline(#[from] AwsmPipelineError),

    #[error("{0}")]
    Light(#[from] AwsmLightError),


    #[error("[post-process] missing post process sampler {0:?}")]
    PostProcess(#[from] AwsmPostProcessError),
}

pub type Result<T> = std::result::Result<T, AwsmError>;
