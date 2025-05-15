use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::{
    configuration::CanvasConfiguration,
    error::{AwsmCoreError, Result},
};

// relatively cheap to clone
#[derive(Clone)]
pub struct AwsmRendererWebGpu {
    pub gpu: web_sys::Gpu,
    pub adapter: web_sys::GpuAdapter,
    pub device: web_sys::GpuDevice,
    pub context: web_sys::GpuCanvasContext,
}

pub struct AwsmRendererWebGpuBuilder {
    pub gpu: web_sys::Gpu,
    pub adapter: Option<web_sys::GpuAdapter>,
    pub device: Option<web_sys::GpuDevice>,
    pub context: Option<web_sys::GpuCanvasContext>,
}

impl AwsmRendererWebGpuBuilder {
    pub fn new(gpu: web_sys::Gpu) -> Self {
        Self {
            gpu,
            adapter: None,
            device: None,
            context: None,
        }
    }

    pub async fn init_adapter(mut self) -> Result<Self> {
        let adapter: web_sys::GpuAdapter = JsFuture::from(self.gpu.request_adapter())
            .await
            .map_err(AwsmCoreError::gpu_adapter)?
            .unchecked_into();

        self.adapter = Some(adapter);

        Ok(self)
    }

    pub async fn init_device(mut self) -> Result<Self> {
        let adapter = self.adapter.as_ref().ok_or(AwsmCoreError::GpuAdapter(
            "Adapter not initialized".to_string(),
        ))?;

        let device: web_sys::GpuDevice = JsFuture::from(adapter.request_device())
            .await
            .map_err(AwsmCoreError::gpu_device)?
            .unchecked_into();

        self.device = Some(device);

        Ok(self)
    }

    pub fn init_context(
        mut self,
        canvas: web_sys::HtmlCanvasElement,
        configuration: Option<CanvasConfiguration>,
    ) -> Result<Self> {
        let device = self.device.as_ref().ok_or(AwsmCoreError::GpuDevice(
            "Device not initialized".to_string(),
        ))?;

        let ctx: web_sys::GpuCanvasContext = match canvas.get_context("webgpu") {
            Ok(Some(ctx)) => Ok(ctx.unchecked_into()),
            Err(err) => Err(AwsmCoreError::canvas_context(err)),
            Ok(None) => Err(AwsmCoreError::CanvasContext("No context found".to_string())),
        }?;

        let configuration = match configuration {
            Some(config) => config,
            None => CanvasConfiguration::new(device, self.gpu.get_preferred_canvas_format()),
        };

        ctx.configure(&configuration.into())
            .map_err(AwsmCoreError::context_configuration)?;

        self.context = Some(ctx);

        Ok(self)
    }

    pub fn build(self) -> Result<AwsmRendererWebGpu> {
        let adapter = self.adapter.ok_or(AwsmCoreError::GpuAdapter(
            "Adapter not initialized".to_string(),
        ))?;
        let device = self.device.ok_or(AwsmCoreError::GpuDevice(
            "Device not initialized".to_string(),
        ))?;
        let context = self.context.ok_or(AwsmCoreError::CanvasContext(
            "Context not initialized".to_string(),
        ))?;

        Ok(AwsmRendererWebGpu {
            gpu: self.gpu,
            adapter,
            device,
            context,
        })
    }
}
