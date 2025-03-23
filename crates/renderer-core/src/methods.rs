use crate::data::JsData;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::{
    command::CommandEncoder,
    error::{AwsmCoreError, Result},
    renderer::AwsmRendererWebGpu,
    texture::TextureFormat,
};

impl AwsmRendererWebGpu {
    pub fn current_context_format(&self) -> TextureFormat {
        self.context
            .get_configuration()
            .as_ref()
            .unwrap()
            .get_format()
    }

    pub fn current_context_texture(&self) -> Result<web_sys::GpuTexture> {
        self.context
            .get_current_texture()
            .map_err(AwsmCoreError::current_context_texture)
    }

    pub fn current_context_texture_view(&self) -> Result<web_sys::GpuTextureView> {
        self.current_context_texture()?
            .create_view()
            .map_err(AwsmCoreError::current_context_texture_view)
    }

    /// Example usage:
    /// let descriptor:ShaderModuleDescriptor = ...;
    /// renderer.compile_shader(&descriptor.into());
    pub fn compile_shader(
        &self,
        shader_code: &web_sys::GpuShaderModuleDescriptor,
    ) -> web_sys::GpuShaderModule {
        self.device.create_shader_module(shader_code)
    }

    /// Example usage:
    /// let descriptor:RenderPipelineDescriptor = ...;
    /// renderer.create_pipeline(&descriptor.into());
    pub async fn create_pipeline(
        &self,
        descriptor: &web_sys::GpuRenderPipelineDescriptor,
    ) -> Result<web_sys::GpuRenderPipeline> {
        let pipeline: web_sys::GpuRenderPipeline =
            JsFuture::from(self.device.create_render_pipeline_async(descriptor))
                .await
                .map_err(AwsmCoreError::pipeline_creation)?
                .unchecked_into();

        Ok(pipeline)
    }

    /// Example usage:
    /// let descriptor:PipelineLayoutDescriptor = ...;
    /// renderer.create_pipeline_layout(&descriptor.into());
    pub fn create_pipeline_layout(
        &self,
        descriptor: &web_sys::GpuPipelineLayoutDescriptor,
    ) -> web_sys::GpuPipelineLayout {
        self.device.create_pipeline_layout(descriptor)
    }

    /// Example usage:
    /// let descriptor:BindGroupLayoutDescriptor = ...;
    /// renderer.create_bind_group_layout(&descriptor.into());
    pub fn create_bind_group_layout(
        &self,
        descriptor: &web_sys::GpuBindGroupLayoutDescriptor,
    ) -> Result<web_sys::GpuBindGroupLayout> {
        self.device
            .create_bind_group_layout(descriptor)
            .map_err(AwsmCoreError::bind_group_layout)
    }

    /// Example usage:
    /// let descriptor:BindGroupDescriptor = ...;
    /// renderer.create_bind_group(&descriptor.into());
    pub fn create_bind_group(
        &self,
        descriptor: &web_sys::GpuBindGroupDescriptor,
    ) -> web_sys::GpuBindGroup {
        self.device.create_bind_group(descriptor)
    }

    /// Example usage:
    /// let descriptor:SamplerDescriptor = ...;
    /// renderer.create_sampler(Some(&descriptor.into()));
    pub fn create_sampler(
        &self,
        descriptor: Option<&web_sys::GpuSamplerDescriptor>,
    ) -> web_sys::GpuSampler {
        match descriptor {
            Some(descriptor) => self.device.create_sampler_with_descriptor(descriptor),
            None => self.device.create_sampler(),
        }
    }

    /// Example usage:
    /// let descriptor:TextureDescriptor = ...;
    /// renderer.create_texture(&descriptor.into());
    pub fn create_texture(
        &self,
        descriptor: &web_sys::GpuTextureDescriptor,
    ) -> Result<web_sys::GpuTexture> {
        self.device
            .create_texture(descriptor)
            .map_err(AwsmCoreError::texture_creation)
    }

    /// Example usage:
    /// let descriptor:BufferDescriptor = ...;
    /// renderer.create_buffer(&descriptor.into());
    pub fn create_buffer(
        &self,
        descriptor: &web_sys::GpuBufferDescriptor,
    ) -> Result<web_sys::GpuBuffer> {
        self.device
            .create_buffer(descriptor)
            .map_err(AwsmCoreError::buffer_creation)
    }

    /// Example usage:
    /// let encoder = renderer.create_command_encoder(Some("My Encoder"));
    /// let render_pass = command_encoder.begin_render_pass(
    ///     &RenderPassDescriptor {
    ///         color_attachments: vec![ColorAttachment::new(
    ///             &renderer.gpu.current_context_texture_view()?,
    ///             LoadOp::Clear,
    ///             StoreOp::Store,
    ///         )],
    ///         ..Default::default()
    ///     }
    ///     .into()
    /// );
    ///
    /// render_pass.set_pipeline(&pipeline);
    /// render_pass.draw(3);
    /// render_pass.end();
    /// self.gpu.submit_commands(&command_encoder.finish());
    pub fn create_command_encoder(&self, label: Option<&str>) -> CommandEncoder {
        let encoder = match label {
            None => self.device.create_command_encoder(),
            Some(label) => {
                let descriptor = web_sys::GpuCommandEncoderDescriptor::new();
                descriptor.set_label(label);
                self.device
                    .create_command_encoder_with_descriptor(&descriptor)
            }
        };

        CommandEncoder::new(encoder)
    }

    /// See [create_command_encoder](create_command_encoder) for usage.
    pub fn submit_commands(&self, command_buffer: &web_sys::GpuCommandBuffer) {
        self.device
            .queue()
            .submit(&js_sys::Array::of1(command_buffer));
    }

    /// See [create_command_encoder](create_command_encoder) for usage.
    pub fn submit_commands_batch(&self, command_buffers: &[&web_sys::GpuCommandBuffer]) {
        let command_buffers_js = js_sys::Array::new();
        for command_buffer in command_buffers {
            command_buffers_js.push(command_buffer);
        }
        self.device.queue().submit(&command_buffers_js);
    }

    // pretty much a direct pass-through, just a bit nicer
    pub fn create_query_set(
        &self,
        query_type: web_sys::GpuQueryType,
        count: u32,
        label: Option<&str>,
    ) -> Result<web_sys::GpuQuerySet> {
        let descriptor = web_sys::GpuQuerySetDescriptor::new(count, query_type);

        if let Some(label) = label {
            descriptor.set_label(label);
        }

        self.device
            .create_query_set(&descriptor)
            .map_err(AwsmCoreError::query_set_creation)
    }

    /// Example usage:
    /// let descriptor:ExternalTextureDescriptor = ...;
    /// renderer.import_external_texture(&descriptor.into());
    pub fn import_external_texture(
        &self,
        descriptor: &web_sys::GpuExternalTextureDescriptor,
    ) -> Result<web_sys::GpuExternalTexture> {
        self.device
            .import_external_texture(descriptor)
            .map_err(AwsmCoreError::external_texture_creation)
    }

    /// Example usage:
    /// let data: &[u8] = ...;
    /// renderer.write_buffer(buffer, None, data, None, None);
    #[allow(private_bounds)]
    pub fn write_buffer<'a>(
        &self,
        buffer: &web_sys::GpuBuffer,
        buffer_offset: Option<u64>,
        data: impl Into<JsData<'a>>,
        data_offset: Option<u64>,
        data_size: Option<u64>,
    ) -> Result<()> {
        // https://developer.mozilla.org/en-US/docs/Web/API/GPUQueue/writeBuffer

        let data = data.into();

        match data {
            JsData::SliceU8(data) => match (data_offset, data_size) {
                (None, None) => self.device.queue().write_buffer_with_f64_and_u8_slice(
                    buffer,
                    buffer_offset.unwrap_or(0) as f64,
                    data,
                ),
                (Some(data_offset), Some(data_size)) => self
                    .device
                    .queue()
                    .write_buffer_with_f64_and_u8_slice_and_f64_and_f64(
                        buffer,
                        buffer_offset.unwrap_or(0) as f64,
                        data,
                        data_offset as f64,
                        data_size as f64,
                    ),
                (Some(data_offset), None) => self
                    .device
                    .queue()
                    .write_buffer_with_f64_and_u8_slice_and_f64(
                        buffer,
                        buffer_offset.unwrap_or(0) as f64,
                        data,
                        data_offset as f64,
                    ),
                (None, Some(data_size)) => self
                    .device
                    .queue()
                    .write_buffer_with_f64_and_u8_slice_and_f64_and_f64(
                        buffer,
                        buffer_offset.unwrap_or(0) as f64,
                        data,
                        0.0,
                        data_size as f64,
                    ),
            },
            _ => match (data_offset, data_size) {
                (None, None) => self.device.queue().write_buffer_with_f64_and_buffer_source(
                    buffer,
                    buffer_offset.unwrap_or(0) as f64,
                    data.as_js_value_ref().unchecked_ref(),
                ),
                (Some(data_offset), Some(data_size)) => self
                    .device
                    .queue()
                    .write_buffer_with_f64_and_buffer_source_and_f64_and_f64(
                        buffer,
                        buffer_offset.unwrap_or(0) as f64,
                        data.as_js_value_ref().unchecked_ref(),
                        data_offset as f64,
                        data_size as f64,
                    ),
                (Some(data_offset), None) => self
                    .device
                    .queue()
                    .write_buffer_with_f64_and_buffer_source_and_f64(
                        buffer,
                        buffer_offset.unwrap_or(0) as f64,
                        data.as_js_value_ref().unchecked_ref(),
                        data_offset as f64,
                    ),
                (None, Some(data_size)) => self
                    .device
                    .queue()
                    .write_buffer_with_f64_and_buffer_source_and_f64_and_f64(
                        buffer,
                        buffer_offset.unwrap_or(0) as f64,
                        data.as_js_value_ref().unchecked_ref(),
                        0.0,
                        data_size as f64,
                    ),
            },
        }
        .map_err(AwsmCoreError::buffer_write)
    }

    /// Example usage:
    /// let destination:TexelCopyTextureInfo = ...;
    /// let layout: TexelCopyBufferLayout = ...;
    /// let size: Extent3d = ...;
    /// let data: &[u8] = ...;
    /// renderer.write_texture(&destination.into(), data, &layout.into(), &size.into());
    #[allow(private_bounds)]
    pub fn write_texture<'a>(
        &self,
        destination: &web_sys::GpuTexelCopyTextureInfo,
        data: impl Into<JsData<'a>>,
        layout: &web_sys::GpuTexelCopyBufferLayout,
        size: &web_sys::GpuExtent3dDict,
    ) -> Result<()> {
        // https://developer.mozilla.org/en-US/docs/Web/API/GPUQueue/writeTexture

        let data = data.into();
        match data {
            JsData::SliceU8(data) => self
                .device
                .queue()
                .write_texture_with_u8_slice_and_gpu_extent_3d_dict(
                    destination,
                    data,
                    layout,
                    size,
                ),
            _ => self
                .device
                .queue()
                .write_texture_with_buffer_source_and_gpu_extent_3d_dict(
                    destination,
                    data.as_js_value_ref().unchecked_ref(),
                    layout,
                    size,
                ),
        }
        .map_err(AwsmCoreError::texture_write)
    }
}
