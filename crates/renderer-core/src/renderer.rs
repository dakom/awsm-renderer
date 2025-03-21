use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::prelude::*;

use crate::error::{AwsmError, Result};


pub struct AwsmRenderer {
    pub gpu: web_sys::Gpu,
    pub adapter: web_sys::GpuAdapter,
    pub device: web_sys::GpuDevice,
    pub context: web_sys::GpuCanvasContext,
}

pub struct AwsmRendererBuilder {
    pub gpu: web_sys::Gpu,
    pub adapter: Option<web_sys::GpuAdapter>,
    pub device: Option<web_sys::GpuDevice>,
    pub context: Option<web_sys::GpuCanvasContext>,
}

impl AwsmRendererBuilder {
    pub fn new(gpu: web_sys::Gpu) -> Self {
        Self {
            gpu,
            adapter: None,
            device: None,
            context: None,
        }
    }

    pub async fn init_adapter(mut self) -> Result<Self> {
        let adapter:web_sys::GpuAdapter = JsFuture::from(self.gpu.request_adapter()).await.map_err(AwsmError::gpu_adapter)?.unchecked_into();

        self.adapter = Some(adapter);

        Ok(self)
    }

    pub async fn init_device(mut self) -> Result<Self> {
        let adapter = self.adapter.as_ref().ok_or(AwsmError::GpuAdapter("Adapter not initialized".to_string()))?;

        let device:web_sys::GpuDevice = JsFuture::from(adapter.request_device()).await.map_err(AwsmError::gpu_device)?.unchecked_into();

        self.device = Some(device);

        Ok(self)
    }

    pub fn init_context(mut self, canvas: web_sys::HtmlCanvasElement) -> Result<Self> {
        let device = self.device.as_ref().ok_or(AwsmError::GpuDevice("Device not initialized".to_string()))?;

        let ctx:web_sys::GpuCanvasContext = match canvas.get_context("webgpu") {
            Ok(Some(ctx)) => Ok(ctx.unchecked_into()),
            Err(err) => {
                Err(AwsmError::canvas_context(err))
            },
            Ok(None) => {
                Err(AwsmError::CanvasContext("No context found".to_string()))
            }
        }?;

        let presentation_format:web_sys::GpuTextureFormat = self.gpu.get_preferred_canvas_format();

        // https://developer.mozilla.org/en-US/docs/Web/API/GPUCanvasContext/configure

        let configuration = web_sys::GpuCanvasConfiguration::new(device, presentation_format);

        ctx.configure(&configuration).map_err(AwsmError::context_configuration)?;

        self.context = Some(ctx);

        Ok(self)
    }

    pub fn build(self) -> Result<AwsmRenderer> {
        let adapter = self.adapter.ok_or(AwsmError::GpuAdapter("Adapter not initialized".to_string()))?;
        let device = self.device.ok_or(AwsmError::GpuDevice("Device not initialized".to_string()))?;
        let context = self.context.ok_or(AwsmError::CanvasContext("Context not initialized".to_string()))?;

        Ok(AwsmRenderer {
            gpu: self.gpu,
            adapter,
            device,
            context,
        })
    }
}