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

pub struct AwsmRendererWebGpuBuilder <'a> {
    pub gpu: web_sys::Gpu,
    pub canvas: web_sys::HtmlCanvasElement,
    pub configuration: Option<CanvasConfiguration<'a>>,
    pub adapter: Option<web_sys::GpuAdapter>,
    pub device: Option<web_sys::GpuDevice>,
    pub context: Option<web_sys::GpuCanvasContext>,
}

impl <'a> AwsmRendererWebGpuBuilder<'a> {
    pub fn new(gpu: web_sys::Gpu, canvas: web_sys::HtmlCanvasElement) -> Self {
        Self {
            gpu,
            canvas,
            configuration: None,
            adapter: None,
            device: None,
            context: None,
        }
    }

    pub fn with_configuration(mut self, configuration: CanvasConfiguration<'a>) -> Self {
        self.configuration = Some(configuration);
        self
    }

    pub fn with_adapter(mut self, adapter: web_sys::GpuAdapter) -> Self {
        self.adapter = Some(adapter);
        self
    }

    pub fn with_device(mut self, device: web_sys::GpuDevice) -> Self {
        self.device = Some(device);
        self
    }

    pub async fn build(self) -> Result<AwsmRendererWebGpu> {
        let adapter: web_sys::GpuAdapter = match self.adapter {
            Some(adapter) => adapter,
            None => {
                JsFuture::from(self.gpu.request_adapter())
                .await
                .map_err(AwsmCoreError::gpu_adapter)?
                .unchecked_into()
            }
        };

        let device:web_sys::GpuDevice = match self.device {
            Some(device) => device,
            None => {
                JsFuture::from(adapter.request_device())
                    .await
                    .map_err(AwsmCoreError::gpu_device)?
                    .unchecked_into()
            }
        };

        let context: web_sys::GpuCanvasContext = match self.canvas.get_context("webgpu") {
            Ok(Some(ctx)) => Ok(ctx.unchecked_into()),
            Err(err) => Err(AwsmCoreError::canvas_context(err)),
            Ok(None) => Err(AwsmCoreError::CanvasContext("No context found".to_string())),
        }?;

        let configuration = match self.configuration {
            Some(config) => config,
            None => CanvasConfiguration::new(&device, self.gpu.get_preferred_canvas_format()),
        };

        context.configure(&configuration.into())
            .map_err(AwsmCoreError::context_configuration)?;

        Ok(AwsmRendererWebGpu {
            gpu: self.gpu,
            adapter,
            device,
            context,
        })
    }
}
