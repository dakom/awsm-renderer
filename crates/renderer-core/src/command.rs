pub mod compute_pass;
pub mod render_pass;
pub mod color;

use std::ops::Deref;

use compute_pass::ComputePassEncoder;
use render_pass::RenderPassEncoder;
use crate::error::{Result, AwsmError};

#[derive(Debug, Clone)]
pub struct CommandEncoder {
    inner: web_sys::GpuCommandEncoder,
}

impl Deref for CommandEncoder {
    type Target = web_sys::GpuCommandEncoder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl CommandEncoder {
    pub fn new(inner: web_sys::GpuCommandEncoder) -> Self {
        Self {
            inner
        }
    }

    pub fn begin_compute_pass(&self, descriptor: Option<&web_sys::GpuComputePassDescriptor>) -> ComputePassEncoder {
        ComputePassEncoder::new(match descriptor {
            Some(descriptor) => self.inner.begin_compute_pass_with_descriptor(descriptor),
            None => self.inner.begin_compute_pass(),
        })
    }

    pub fn begin_render_pass(&self, descriptor: &web_sys::GpuRenderPassDescriptor) -> Result<RenderPassEncoder> {
        Ok(RenderPassEncoder::new(
            self.inner.begin_render_pass(descriptor).map_err(AwsmError::command_render_pass)?,
        ))
    }

    // TODO - add more methods for command encoder
    // all of https://developer.mozilla.org/en-US/docs/Web/API/GPUCommandEncoder#instance_methods

    pub fn finish(self) -> web_sys::GpuCommandBuffer {
        self.inner.finish()
    }
}

pub type LoadOp = web_sys::GpuLoadOp;
pub type StoreOp = web_sys::GpuStoreOp;