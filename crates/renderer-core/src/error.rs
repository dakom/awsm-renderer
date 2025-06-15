use thiserror::Error;
use wasm_bindgen::prelude::*;

use crate::shaders::ShaderCompilationMessage;

pub type Result<T> = std::result::Result<T, AwsmCoreError>;

#[derive(Error, Debug)]
pub enum AwsmCoreError {
    #[error("[gpu] Failed to create Adapter: {0}")]
    GpuAdapter(String),

    #[error("[gpu] Failed to create Device: {0}")]
    GpuDevice(String),

    #[error("[gpu] Failed to create Canvas Context: {0}")]
    CanvasContext(String),

    #[error("[gpu] Failed to configure Context: {0}")]
    ContextConfiguration(String),

    #[error("[gpu] Failed to create Pipeline from valid descriptor: {0}")]
    PipelineCreation(String),

    #[error("[gpu] Failed to create Pipeline Descriptor: {0}")]
    PipelineDescriptor(String),

    #[error("[gpu] Shader not found")]
    ShaderNotFound,

    #[error("[gpu] Failed to create query set: {0}")]
    QuerySetCreation(String),

    #[error("[gpu] Failed to create Bind Group Layout: {0}")]
    BindGroupLayout(String),

    #[error("[gpu] Failed to create External Texture: {0}")]
    ExternalTextureCreation(String),

    #[error("[gpu] Failed to create Texture: {0}")]
    TextureCreation(String),

    #[error("[gpu] Failed to create RenderPass Command: {0}")]
    CommandRenderPass(String),

    #[error("[gpu] Failed to get current context texture: {0}")]
    CurrentContextTexture(String),

    #[error("[gpu] Failed to get current context texture view: {0}")]
    CurrentContextTextureView(String),

    #[error("[gpu] failed copy buffer to buffer command: {0}")]
    CommandCopyBufferToBuffer(String),

    #[error("[gpu] failed copy buffer to texture command: {0}")]
    CommandCopyBufferToTexture(String),

    #[error("[gpu] failed copy texture to buffer command: {0}")]
    CommandCopyTextureToBuffer(String),

    #[error("[gpu] failed copy texture to texture command: {0}")]
    CommandCopyTextureToTexture(String),

    #[error("[gpu] failed create buffer: {0}")]
    BufferCreation(String),

    #[error("[gpu] failed write buffer: {0}")]
    BufferWrite(String),

    #[error("[gpu] failed write texture: {0}")]
    TextureWrite(String),

    #[cfg(feature = "image")]
    #[error("[gpu] Image load: {0}")]
    ImageLoad(String),

    #[cfg(feature = "image")]
    #[error("[gpu] Failed to get location origin: {0}")]
    LocationOrigin(String),

    #[cfg(feature = "image")]
    #[error("[gpu] Failed to parse url: {0}")]
    UrlParse(String),

    #[cfg(feature = "image")]
    #[error("[gpu] Failed to copy external image to texture: {0}")]
    CopyExternalImageToTexture(String),

    #[cfg(feature = "exr")]
    #[error("[gpu] Failed to create js value from exr image data: {0}")]
    ExrImageToJsValue(String),

    #[error("[gpu] Failed to get Shader compilation info: {0}")]
    ShaderCompilationInfo(String),

    #[error("[gpu] Failed to validate Shader: {0:#?}")]
    ShaderValidation(Vec<ShaderCompilationMessage>),

    #[error("[gpu] Failed to set bind group: {0}")]
    SetBindGroup(String),

    #[error("[gpu] Failed to fetch: {0:?}")]
    Fetch(String),

    #[error("[gpu] Failed to create image bitmap: {0:?}")]
    CreateImageBitmap(String),

    #[error("[gpu] Failed to create texture: {0:?}")]
    CreateTexture(String),

    #[error("[gpu] Failed to create texture view: {0:?}")]
    CreateTextureView(String),
}

impl AwsmCoreError {
    pub fn gpu_adapter(err: JsValue) -> Self {
        Self::GpuAdapter(format_err(err))
    }

    pub fn gpu_device(err: JsValue) -> Self {
        Self::GpuDevice(format_err(err))
    }

    pub fn canvas_context(err: JsValue) -> Self {
        Self::CanvasContext(format_err(err))
    }

    pub fn context_configuration(err: JsValue) -> Self {
        Self::ContextConfiguration(format_err(err))
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
            Err(err) => Self::PipelineCreation(format_err(err)),
        }
    }

    pub fn pipeline_descriptor(err: JsValue) -> Self {
        Self::PipelineDescriptor(format_err(err))
    }

    pub fn query_set_creation(err: JsValue) -> Self {
        Self::QuerySetCreation(format_err(err))
    }

    pub fn bind_group_layout(err: JsValue) -> Self {
        Self::BindGroupLayout(format_err(err))
    }

    pub fn external_texture_creation(err: JsValue) -> Self {
        Self::ExternalTextureCreation(format_err(err))
    }

    pub fn texture_creation(err: JsValue) -> Self {
        Self::TextureCreation(format_err(err))
    }

    pub fn command_render_pass(err: JsValue) -> Self {
        Self::CommandRenderPass(format_err(err))
    }

    pub fn current_context_texture(err: JsValue) -> Self {
        Self::CurrentContextTexture(format_err(err))
    }

    pub fn current_context_texture_view(err: JsValue) -> Self {
        Self::CurrentContextTextureView(format_err(err))
    }

    pub fn command_copy_buffer_to_buffer(err: JsValue) -> Self {
        Self::CommandCopyBufferToBuffer(format_err(err))
    }

    pub fn command_copy_buffer_to_texture(err: JsValue) -> Self {
        Self::CommandCopyBufferToTexture(format_err(err))
    }

    pub fn command_copy_texture_to_buffer(err: JsValue) -> Self {
        Self::CommandCopyTextureToBuffer(format_err(err))
    }

    pub fn command_copy_texture_to_texture(err: JsValue) -> Self {
        Self::CommandCopyTextureToTexture(format_err(err))
    }

    pub fn buffer_creation(err: JsValue) -> Self {
        Self::BufferCreation(format_err(err))
    }

    pub fn buffer_write(err: JsValue) -> Self {
        Self::BufferWrite(format_err(err))
    }

    pub fn texture_write(err: JsValue) -> Self {
        Self::TextureWrite(format_err(err))
    }

    #[cfg(feature = "image")]
    pub fn image_load(err: JsValue) -> Self {
        Self::ImageLoad(format_err(err))
    }

    #[cfg(feature = "image")]
    pub fn location_origin(err: JsValue) -> Self {
        Self::LocationOrigin(format_err(err))
    }

    #[cfg(feature = "image")]
    pub fn url_parse(err: JsValue) -> Self {
        Self::UrlParse(format_err(err))
    }

    #[cfg(feature = "image")]
    pub fn copy_external_image_to_texture(err: JsValue) -> Self {
        Self::CopyExternalImageToTexture(format_err(err))
    }

    #[cfg(feature = "exr")]
    pub fn exr_image_to_js_value(err: JsValue) -> Self {
        Self::ExrImageToJsValue(format_err(err))
    }

    pub fn shader_compilation_info(err: JsValue) -> Self {
        Self::ShaderCompilationInfo(format_err(err))
    }

    pub fn set_bind_group(err: JsValue) -> Self {
        Self::SetBindGroup(format_err(err))
    }

    pub fn fetch(err: JsValue) -> Self {
        Self::Fetch(format_err(err))
    }

    pub fn create_image_bitmap(err: JsValue) -> Self {
        Self::CreateImageBitmap(format_err(err))
    }

    pub fn create_texture(err: JsValue) -> Self {
        Self::CreateTexture(format_err(err))
    }

    pub fn create_texture_view(err: JsValue) -> Self {
        Self::CreateTextureView(format_err(err))
    }
}

fn format_err(err: JsValue) -> String {
    err.as_string().unwrap_or_else(|| format!("{:#?}", err))
}
