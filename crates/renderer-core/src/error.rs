use thiserror::Error;
use wasm_bindgen::prelude::*;

pub type Result<T> = std::result::Result<T, AwsmError>;

#[derive(Error, Debug)]
pub enum AwsmError {
    #[error("Failed to create GPU Adapter: {0}")]
    GpuAdapter(String),
    #[error("Failed to create GPU Device: {0}")]
    GpuDevice(String),
    #[error("Failed to create Canvas WebGPU Context: {0}")]
    CanvasContext(String),
    #[error("Failed to configure WebGPU Context: {0}")]
    ContextConfiguration(String),
    #[error("Failed to create WebGPU Pipeline from valid descriptor: {0}")]
    PipelineCreation(String),
    #[error("Failed to create WebGPU Pipeline Descriptor: {0}")]
    PipelineDescriptor(String),
    #[error("Shader not found")]
    ShaderNotFound,
    #[error("Failed to create WebGPU query set: {0}")]
    QuerySetCreation(String),
    #[error("Failed to create WebGPU Bind Group Layout: {0}")]
    BindGroupLayout(String),
    #[error("Failed to create WebGPU External Texture: {0}")]
    ExternalTextureCreation(String),
    #[error("Failed to create WebGPU Texture: {0}")]
    TextureCreation(String),
    #[error("Failed to create WebGPU RenderPass Command: {0}")]
    CommandRenderPass(String),
    #[error("Failed to get WebGPU current context texture: {0}")]
    CurrentContextTexture(String),
    #[error("Failed to get WebGPU current context texture view: {0}")]
    CurrentContextTextureView(String),
}

impl AwsmError {
    pub fn gpu_adapter(err: JsValue) -> Self {
        Self::GpuAdapter(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn gpu_device(err: JsValue) -> Self {
        Self::GpuDevice(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn canvas_context(err: JsValue) -> Self {
        Self::CanvasContext(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn context_configuration(err: JsValue) -> Self {
        Self::ContextConfiguration(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn pipeline_creation(err: JsValue) -> Self {
        match err.dyn_into::<web_sys::GpuPipelineError>() {
            Ok(err) => {
                let reason = match err.reason() {
                    web_sys::GpuPipelineErrorReason::Validation => {
                        "Validation"
                    },
                    web_sys::GpuPipelineErrorReason::Internal => {
                        "Internal"
                    },
                    _ => {
                        "Unknown"
                    }
                };

                Self::PipelineCreation(format!("Pipeline creation [{}] error: {}", reason, err.message()))
            }
            Err(err) => {
                Self::PipelineCreation(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
            }
        }
    }

    pub fn pipeline_descriptor(err: JsValue) -> Self {
        Self::PipelineDescriptor(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn query_set_creation(err: JsValue) -> Self {
        Self::QuerySetCreation(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn bind_group_layout(err: JsValue) -> Self {
        Self::BindGroupLayout(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn external_texture_creation(err: JsValue) -> Self {
        Self::ExternalTextureCreation(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn texture_creation(err: JsValue) -> Self {
        Self::TextureCreation(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn command_render_pass(err: JsValue) -> Self {
        Self::CommandRenderPass(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn current_context_texture(err: JsValue) -> Self {
        Self::CurrentContextTexture(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }

    pub fn current_context_texture_view(err: JsValue) -> Self {
        Self::CurrentContextTextureView(err.as_string().unwrap_or_else(|| "Unknown error".to_string()))
    }
}
