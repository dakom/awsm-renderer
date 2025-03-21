use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::{
    command::CommandEncoder,
    error::{AwsmError, Result},
    renderer::AwsmRenderer,
    texture::TextureFormat,
};

impl AwsmRenderer {
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
            .map_err(AwsmError::current_context_texture)
    }

    pub fn current_context_texture_view(&self) -> Result<web_sys::GpuTextureView> {
        self.current_context_texture()?
            .create_view()
            .map_err(AwsmError::current_context_texture_view)
    }

    pub fn compile_shader(
        &self,
        shader_code: &web_sys::GpuShaderModuleDescriptor,
    ) -> web_sys::GpuShaderModule {
        self.device.create_shader_module(&shader_code)
    }

    pub async fn create_pipeline(
        &self,
        descriptor: &web_sys::GpuRenderPipelineDescriptor,
    ) -> Result<web_sys::GpuRenderPipeline> {
        let pipeline: web_sys::GpuRenderPipeline =
            JsFuture::from(self.device.create_render_pipeline_async(descriptor))
                .await
                .map_err(AwsmError::pipeline_creation)?
                .unchecked_into();

        Ok(pipeline)
    }

    pub fn create_pipeline_layout(
        &self,
        descriptor: &web_sys::GpuPipelineLayoutDescriptor,
    ) -> web_sys::GpuPipelineLayout {
        self.device.create_pipeline_layout(descriptor)
    }

    pub fn create_bind_group_layout(
        &self,
        descriptor: &web_sys::GpuBindGroupLayoutDescriptor,
    ) -> Result<web_sys::GpuBindGroupLayout> {
        self.device
            .create_bind_group_layout(descriptor)
            .map_err(AwsmError::bind_group_layout)
    }

    pub fn create_bind_group(
        &self,
        descriptor: &web_sys::GpuBindGroupDescriptor,
    ) -> web_sys::GpuBindGroup {
        self.device.create_bind_group(descriptor)
    }

    pub fn create_sampler(
        &self,
        descriptor: Option<&web_sys::GpuSamplerDescriptor>,
    ) -> web_sys::GpuSampler {
        match descriptor {
            Some(descriptor) => self.device.create_sampler_with_descriptor(descriptor),
            None => self.device.create_sampler(),
        }
    }

    pub fn create_texture(
        &self,
        descriptor: &web_sys::GpuTextureDescriptor,
    ) -> Result<web_sys::GpuTexture> {
        self.device
            .create_texture(descriptor)
            .map_err(AwsmError::texture_creation)
    }

    pub fn create_command_encoder(&self, label: Option<&str>) -> CommandEncoder {
        let encoder = match label {
            None => self.device.create_command_encoder(),
            Some(label) => {
                let descriptor = web_sys::GpuCommandEncoderDescriptor::new();
                descriptor.set_label(&label);
                self.device
                    .create_command_encoder_with_descriptor(&descriptor)
            }
        };

        CommandEncoder::new(encoder)
    }

    pub fn submit_commands(&self, command_buffer: &web_sys::GpuCommandBuffer) {
        self.device
            .queue()
            .submit(&js_sys::Array::of1(command_buffer));
    }

    pub fn submit_commands_batch(&self, command_buffers: &[&web_sys::GpuCommandBuffer]) {
        let command_buffers_js = js_sys::Array::new();
        for command_buffer in command_buffers {
            command_buffers_js.push(command_buffer);
        }
        self.device.queue().submit(&command_buffers_js);
    }

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
            .map_err(AwsmError::query_set_creation)
    }

    pub fn import_external_texture(
        &self,
        descriptor: &web_sys::GpuExternalTextureDescriptor,
    ) -> Result<web_sys::GpuExternalTexture> {
        self.device
            .import_external_texture(descriptor)
            .map_err(AwsmError::external_texture_creation)
    }
}
