use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::GpuSupportedLimits;

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
    pub canvas: web_sys::HtmlCanvasElement,
    pub configuration: Option<CanvasConfiguration>,
    pub adapter: Option<web_sys::GpuAdapter>,
    pub device: Option<web_sys::GpuDevice>,
    pub context: Option<web_sys::GpuCanvasContext>,
    pub device_req_limits: Option<DeviceRequestLimits>,
}

impl AwsmRendererWebGpuBuilder {
    pub fn new(gpu: web_sys::Gpu, canvas: web_sys::HtmlCanvasElement) -> Self {
        Self {
            gpu,
            canvas,
            configuration: None,
            adapter: None,
            device: None,
            context: None,
            device_req_limits: None,
        }
    }

    pub fn with_configuration(mut self, configuration: CanvasConfiguration) -> Self {
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

    pub fn with_device_request_limits(mut self, device_req_limits: DeviceRequestLimits) -> Self {
        self.device_req_limits = Some(device_req_limits);
        self
    }

    pub async fn build(self) -> Result<AwsmRendererWebGpu> {
        let adapter: web_sys::GpuAdapter = match self.adapter {
            Some(adapter) => adapter,
            None => JsFuture::from(self.gpu.request_adapter())
                .await
                .map_err(AwsmCoreError::gpu_adapter)?
                .unchecked_into(),
        };

        let device: web_sys::GpuDevice = match self.device {
            Some(device) => device,
            None => {
                if let Some(limits) = self.device_req_limits {
                    let descriptor = web_sys::GpuDeviceDescriptor::new();
                    descriptor.set_required_limits(&limits.into_js(&adapter.limits()));
                    JsFuture::from(adapter.request_device_with_descriptor(&descriptor))
                        .await
                        .map_err(AwsmCoreError::gpu_device)?
                        .unchecked_into()
                } else {
                    JsFuture::from(adapter.request_device())
                    .await
                    .map_err(AwsmCoreError::gpu_device)?
                    .unchecked_into()
                }
            }
        };

        let context: web_sys::GpuCanvasContext = match self.canvas.get_context("webgpu") {
            Ok(Some(ctx)) => Ok(ctx.unchecked_into()),
            Err(err) => Err(AwsmCoreError::canvas_context(err)),
            Ok(None) => Err(AwsmCoreError::CanvasContext("No context found".to_string())),
        }?;

        context
            .configure(
                &self
                    .configuration
                    .unwrap_or_default()
                    .into_js(&self.gpu, &device),
            )
            .map_err(AwsmCoreError::context_configuration)?;

        Ok(AwsmRendererWebGpu {
            gpu: self.gpu,
            adapter,
            device,
            context,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeviceRequestLimits {
    pub max_texture_dimension_2d: bool,
    pub max_texture_array_layers: bool,
    pub max_bindings_per_bind_group: bool,
    pub max_sampled_textures_per_shader_stage: bool,
    pub max_buffer_size: bool,
    pub max_bind_groups: bool,
}

impl DeviceRequestLimits {
    pub fn max_all() -> Self {
        Self {
            max_texture_dimension_2d: true,
            max_texture_array_layers: true,
            max_bindings_per_bind_group: true,
            max_sampled_textures_per_shader_stage: true,
            max_buffer_size: true,
            max_bind_groups: true
        }
    }

    pub fn into_js(self, limits:&GpuSupportedLimits) -> js_sys::Object {
        let obj = js_sys::Object::new(); 

        if self.max_texture_dimension_2d {
            js_sys::Reflect::set(&obj, &"maxTextureDimension2D".into(), &JsValue::from_f64(limits.max_texture_dimension_2d() as f64)).unwrap();
        }
        if self.max_texture_array_layers {
            js_sys::Reflect::set(&obj, &"maxTextureArrayLayers".into(), &JsValue::from_f64(limits.max_texture_array_layers() as f64)).unwrap();
        }
        if self.max_bindings_per_bind_group {
            js_sys::Reflect::set(&obj, &"maxBindingsPerBindGroup".into(), &JsValue::from_f64(limits.max_bindings_per_bind_group() as f64)).unwrap();
        }
        if self.max_bind_groups {
            js_sys::Reflect::set(&obj, &"maxBindGroups".into(), &JsValue::from_f64(limits.max_bind_groups() as f64)).unwrap();
        }
        if self.max_sampled_textures_per_shader_stage {
            js_sys::Reflect::set(&obj, &"maxSampledTexturesPerShaderStage".into(), &JsValue::from_f64(limits.max_sampled_textures_per_shader_stage() as f64)).unwrap();
        }
        if self.max_buffer_size {
            js_sys::Reflect::set(&obj, &"maxBufferSize".into(), &JsValue::from_f64(limits.max_buffer_size() as f64)).unwrap();
        }


        obj
    }
}