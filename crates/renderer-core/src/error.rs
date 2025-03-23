use std::sync::LazyLock;

use thiserror::Error;
use wasm_bindgen::prelude::*;

use crate::shaders::ShaderCompilationMessage;

pub type Result<T> = std::result::Result<T, AwsmCoreError>;

#[derive(Error, Debug)]
pub enum AwsmCoreError {
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

    #[error("WebGPU failed copy buffer to buffer command: {0}")]
    CommandCopyBufferToBuffer(String),

    #[error("WebGPU failed copy buffer to texture command: {0}")]
    CommandCopyBufferToTexture(String),

    #[error("WebGPU failed copy texture to buffer command: {0}")]
    CommandCopyTextureToBuffer(String),

    #[error("WebGPU failed copy texture to texture command: {0}")]
    CommandCopyTextureToTexture(String),

    #[error("WebGPU failed create buffer: {0}")]
    BufferCreation(String),

    #[error("WebGPU failed write buffer: {0}")]
    BufferWrite(String),

    #[error("WebGPU failed write texture: {0}")]
    TextureWrite(String),

    #[cfg(feature = "image")]
    #[error("Image load: {0}")]
    ImageLoad(String),

    #[cfg(feature = "image")]
    #[error("Failed to get location origin: {0}")]
    LocationOrigin(String),

    #[cfg(feature = "image")]
    #[error("Failed to parse url: {0}")]
    UrlParse(String),

    #[error("Failed to get WebGPU Shader compilation info: {0}")]
    ShaderCompilationInfo(String),

    #[error("Failed to validate WebGPU Shader: {0:#?}")]
    ShaderValidation(Vec<ShaderCompilationMessage>),
}

static ERROR_UNKNOWN: LazyLock<String> = LazyLock::new(|| "Unknown error".to_string());

impl AwsmCoreError {
    pub fn gpu_adapter(err: JsValue) -> Self {
        Self::GpuAdapter(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn gpu_device(err: JsValue) -> Self {
        Self::GpuDevice(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn canvas_context(err: JsValue) -> Self {
        Self::CanvasContext(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn context_configuration(err: JsValue) -> Self {
        Self::ContextConfiguration(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn pipeline_creation(err: JsValue) -> Self {
        match err.dyn_into::<web_sys::GpuPipelineError>() {
            Ok(err) => {
                let reason = match err.reason() {
                    web_sys::GpuPipelineErrorReason::Validation => "Validation",
                    web_sys::GpuPipelineErrorReason::Internal => "Internal",
                    _ => "Unknown",
                };

                Self::PipelineCreation(format!(
                    "Pipeline creation [{}] error: {}",
                    reason,
                    err.message()
                ))
            }
            Err(err) => {
                Self::PipelineCreation(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
            }
        }
    }

    pub fn pipeline_descriptor(err: JsValue) -> Self {
        Self::PipelineDescriptor(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn query_set_creation(err: JsValue) -> Self {
        Self::QuerySetCreation(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn bind_group_layout(err: JsValue) -> Self {
        Self::BindGroupLayout(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn external_texture_creation(err: JsValue) -> Self {
        Self::ExternalTextureCreation(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn texture_creation(err: JsValue) -> Self {
        Self::TextureCreation(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn command_render_pass(err: JsValue) -> Self {
        Self::CommandRenderPass(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn current_context_texture(err: JsValue) -> Self {
        Self::CurrentContextTexture(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn current_context_texture_view(err: JsValue) -> Self {
        Self::CurrentContextTextureView(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn command_copy_buffer_to_buffer(err: JsValue) -> Self {
        Self::CommandCopyBufferToBuffer(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn command_copy_buffer_to_texture(err: JsValue) -> Self {
        Self::CommandCopyBufferToTexture(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn command_copy_texture_to_buffer(err: JsValue) -> Self {
        Self::CommandCopyTextureToBuffer(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn command_copy_texture_to_texture(err: JsValue) -> Self {
        Self::CommandCopyTextureToTexture(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn buffer_creation(err: JsValue) -> Self {
        Self::BufferCreation(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn buffer_write(err: JsValue) -> Self {
        Self::BufferWrite(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn texture_write(err: JsValue) -> Self {
        Self::TextureWrite(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    #[cfg(feature = "image")]
    pub fn image_load(err: JsValue) -> Self {
        Self::ImageLoad(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    #[cfg(feature = "image")]
    pub fn location_origin(err: JsValue) -> Self {
        Self::LocationOrigin(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    #[cfg(feature = "image")]
    pub fn url_parse(err: JsValue) -> Self {
        Self::UrlParse(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }

    pub fn shader_compilation_info(err: JsValue) -> Self {
        Self::ShaderCompilationInfo(err.as_string().unwrap_or_else(|| ERROR_UNKNOWN.clone()))
    }
}
