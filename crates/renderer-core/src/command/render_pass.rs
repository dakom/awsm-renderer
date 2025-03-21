use std::ops::Deref;

use super::{color::Color, LoadOp, StoreOp};

#[derive(Debug, Clone)]
pub struct RenderPassEncoder {
    inner: web_sys::GpuRenderPassEncoder,
}

impl RenderPassEncoder {
    pub fn new(inner: web_sys::GpuRenderPassEncoder) -> Self {
        Self { inner }
    }
}

impl Deref for RenderPassEncoder {
    type Target = web_sys::GpuRenderPassEncoder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenderPassDescriptor<'a> {
    pub color_attachments: Vec<ColorAttachment<'a>>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment<'a>>,
    pub label: Option<&'a str>,
    pub max_draw_count: Option<f64>,
    pub occlusion_query_set: Option<&'a web_sys::GpuQuerySet>,
    pub timestamp_writes: Option<RenderTimestampWrites<'a>>,
}

#[derive(Debug, Clone)]
pub struct ColorAttachment<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUCommandEncoder/beginRenderPass#color_attachment_object_structure
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuRenderPassColorAttachment.html
    pub clear_value: Option<Color>,
    pub depth_slice: Option<u32>,
    pub resolve_target: Option<&'a web_sys::GpuTextureView>,
    pub load_op: LoadOp,
    pub store_op: StoreOp,
    pub view: &'a web_sys::GpuTextureView,
}

impl<'a> ColorAttachment<'a> {
    pub fn new(view: &'a web_sys::GpuTextureView, load_op: LoadOp, store_op: StoreOp) -> Self {
        Self {
            view,
            load_op,
            store_op,
            clear_value: None,
            depth_slice: None,
            resolve_target: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DepthStencilAttachment<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUCommandEncoder/beginRenderPass#depthstencil_attachment_object_structure
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuRenderPassDepthStencilAttachment.html
    pub view: &'a web_sys::GpuTextureView,
    pub depth_clear_value: Option<f32>,
    pub depth_load_op: Option<LoadOp>,
    pub depth_read_only: Option<bool>,
    pub depth_store_op: Option<StoreOp>,
    pub stencil_clear_value: Option<u32>,
    pub stencil_load_op: Option<LoadOp>,
    pub stencil_read_only: Option<bool>,
    pub stencil_store_op: Option<StoreOp>,
}

#[derive(Debug, Clone)]
pub struct RenderTimestampWrites<'a> {
    pub query_set: &'a web_sys::GpuQuerySet,
    pub beginning_index: Option<u32>,
    pub end_index: Option<u32>,
}

// js conversions

impl From<RenderPassDescriptor<'_>> for web_sys::GpuRenderPassDescriptor {
    fn from(pass: RenderPassDescriptor) -> web_sys::GpuRenderPassDescriptor {
        let color_attachments = js_sys::Array::new();
        for attachment in pass.color_attachments {
            color_attachments.push(&web_sys::GpuRenderPassColorAttachment::from(attachment));
        }

        let pass_js = web_sys::GpuRenderPassDescriptor::new(&color_attachments);

        if let Some(label) = pass.label {
            pass_js.set_label(label);
        }

        if let Some(depth_stencil_attachment) = pass.depth_stencil_attachment {
            pass_js.set_depth_stencil_attachment(
                &web_sys::GpuRenderPassDepthStencilAttachment::from(depth_stencil_attachment),
            );
        }

        if let Some(max_draw_count) = pass.max_draw_count {
            pass_js.set_max_draw_count(max_draw_count);
        }

        if let Some(occlusion_query_set) = pass.occlusion_query_set {
            pass_js.set_occlusion_query_set(occlusion_query_set);
        }

        if let Some(timestamp_writes) = pass.timestamp_writes {
            pass_js.set_timestamp_writes(&web_sys::GpuRenderPassTimestampWrites::from(
                timestamp_writes,
            ));
        }

        pass_js
    }
}

impl From<ColorAttachment<'_>> for web_sys::GpuRenderPassColorAttachment {
    fn from(attachment: ColorAttachment) -> web_sys::GpuRenderPassColorAttachment {
        let attachment_js = web_sys::GpuRenderPassColorAttachment::new(
            attachment.load_op,
            attachment.store_op,
            attachment.view,
        );

        if let Some(clear_value) = attachment.clear_value {
            attachment_js.set_clear_value(&clear_value.as_js_value());
        }
        if let Some(depth_slice) = attachment.depth_slice {
            attachment_js.set_depth_slice(depth_slice);
        }
        if let Some(resolve_target) = attachment.resolve_target {
            attachment_js.set_resolve_target(resolve_target);
        }

        attachment_js
    }
}

impl From<DepthStencilAttachment<'_>> for web_sys::GpuRenderPassDepthStencilAttachment {
    fn from(attachment: DepthStencilAttachment) -> web_sys::GpuRenderPassDepthStencilAttachment {
        let attachment_js = web_sys::GpuRenderPassDepthStencilAttachment::new(attachment.view);

        if let Some(depth_clear_value) = attachment.depth_clear_value {
            attachment_js.set_depth_clear_value(depth_clear_value);
        }
        if let Some(depth_load_op) = attachment.depth_load_op {
            attachment_js.set_depth_load_op(depth_load_op);
        }
        if let Some(depth_read_only) = attachment.depth_read_only {
            attachment_js.set_depth_read_only(depth_read_only);
        }
        if let Some(depth_store_op) = attachment.depth_store_op {
            attachment_js.set_depth_store_op(depth_store_op);
        }
        if let Some(stencil_clear_value) = attachment.stencil_clear_value {
            attachment_js.set_stencil_clear_value(stencil_clear_value);
        }
        if let Some(stencil_load_op) = attachment.stencil_load_op {
            attachment_js.set_stencil_load_op(stencil_load_op);
        }
        if let Some(stencil_read_only) = attachment.stencil_read_only {
            attachment_js.set_stencil_read_only(stencil_read_only);
        }
        if let Some(stencil_store_op) = attachment.stencil_store_op {
            attachment_js.set_stencil_store_op(stencil_store_op);
        }

        attachment_js
    }
}

impl From<RenderTimestampWrites<'_>> for web_sys::GpuRenderPassTimestampWrites {
    fn from(timestamp_writes: RenderTimestampWrites) -> web_sys::GpuRenderPassTimestampWrites {
        let timestamp_writes_js =
            web_sys::GpuRenderPassTimestampWrites::new(timestamp_writes.query_set);

        if let Some(beginning_index) = timestamp_writes.beginning_index {
            timestamp_writes_js.set_beginning_of_pass_write_index(beginning_index);
        }
        if let Some(end_index) = timestamp_writes.end_index {
            timestamp_writes_js.set_end_of_pass_write_index(end_index);
        }

        timestamp_writes_js
    }
}
