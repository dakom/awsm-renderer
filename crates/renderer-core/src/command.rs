pub mod color;
pub mod compute_pass;
pub mod copy_texture;
pub mod render_pass;

use std::ops::Deref;

use crate::error::{AwsmCoreError, Result};
use compute_pass::ComputePassEncoder;
use render_pass::RenderPassEncoder;

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
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUCommandEncoder#instance_methods
    pub fn new(inner: web_sys::GpuCommandEncoder) -> Self {
        Self { inner }
    }

    pub fn begin_compute_pass(
        &self,
        descriptor: Option<&web_sys::GpuComputePassDescriptor>,
    ) -> ComputePassEncoder {
        ComputePassEncoder::new(match descriptor {
            Some(descriptor) => self.inner.begin_compute_pass_with_descriptor(descriptor),
            None => self.inner.begin_compute_pass(),
        })
    }

    pub fn begin_render_pass(
        &self,
        descriptor: &web_sys::GpuRenderPassDescriptor,
    ) -> Result<RenderPassEncoder> {
        Ok(RenderPassEncoder::new(
            self.inner
                .begin_render_pass(descriptor)
                .map_err(AwsmCoreError::command_render_pass)?,
        ))
    }

    pub fn clear_buffer(
        &self,
        buffer: &web_sys::GpuBuffer,
        offset: Option<u32>,
        size: Option<u32>,
    ) {
        match (offset, size) {
            (Some(offset), Some(size)) => self
                .inner
                .clear_buffer_with_u32_and_u32(buffer, offset, size),
            (Some(offset), None) => self.inner.clear_buffer_with_u32(buffer, offset),
            (None, Some(size)) => self.inner.clear_buffer_with_u32_and_u32(buffer, 0, size),
            (None, None) => self.inner.clear_buffer(buffer),
        }
    }

    pub fn copy_buffer_to_buffer(
        &self,
        source: &web_sys::GpuBuffer,
        source_offset: u32,
        destination: &web_sys::GpuBuffer,
        destination_offset: u32,
        size: u32,
    ) -> Result<()> {
        self.inner
            .copy_buffer_to_buffer_with_u32_and_u32_and_u32(
                source,
                source_offset,
                destination,
                destination_offset,
                size,
            )
            .map_err(AwsmCoreError::command_copy_buffer_to_buffer)
    }

    pub fn copy_buffer_to_texture(
        &self,
        source: &web_sys::GpuTexelCopyBufferInfo,
        destination: &web_sys::GpuTexelCopyTextureInfo,
        copy_size: &web_sys::GpuExtent3dDict,
    ) -> Result<()> {
        self.inner
            .copy_buffer_to_texture_with_gpu_extent_3d_dict(source, destination, copy_size)
            .map_err(AwsmCoreError::command_copy_buffer_to_texture)
    }

    pub fn copy_texture_to_buffer(
        &self,
        source: &web_sys::GpuTexelCopyTextureInfo,
        destination: &web_sys::GpuTexelCopyBufferInfo,
        copy_size: &web_sys::GpuExtent3dDict,
    ) -> Result<()> {
        self.inner
            .copy_texture_to_buffer_with_gpu_extent_3d_dict(source, destination, copy_size)
            .map_err(AwsmCoreError::command_copy_texture_to_buffer)
    }

    pub fn copy_texture_to_texture(
        &self,
        source: &web_sys::GpuTexelCopyTextureInfo,
        destination: &web_sys::GpuTexelCopyTextureInfo,
        copy_size: &web_sys::GpuExtent3dDict,
    ) -> Result<()> {
        self.inner
            .copy_texture_to_texture_with_gpu_extent_3d_dict(source, destination, copy_size)
            .map_err(AwsmCoreError::command_copy_texture_to_texture)
    }

    pub fn resolve_query_set(
        &self,
        query_set: &web_sys::GpuQuerySet,
        first_query: u32,
        query_count: u32,
        destination: &web_sys::GpuBuffer,
        destination_offset: u32,
    ) {
        self.inner.resolve_query_set_with_u32(
            query_set,
            first_query,
            query_count,
            destination,
            destination_offset,
        );
    }

    pub fn insert_debug_marker(&self, label: &str) {
        self.inner.insert_debug_marker(label);
    }

    pub fn push_debug_group(&self, group_label: &str) {
        self.inner.push_debug_group(group_label);
    }

    pub fn pop_debug_group(&self) {
        self.inner.pop_debug_group();
    }

    pub fn finish(&self) -> web_sys::GpuCommandBuffer {
        self.inner.finish()
    }
}

pub type LoadOp = web_sys::GpuLoadOp;
pub type StoreOp = web_sys::GpuStoreOp;
