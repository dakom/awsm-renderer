use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct ComputePassEncoder {
    inner: web_sys::GpuComputePassEncoder,
}

impl ComputePassEncoder {
    pub fn new(inner: web_sys::GpuComputePassEncoder) -> Self {
        Self { inner }
    }
}

impl Deref for ComputePassEncoder {
    type Target = web_sys::GpuComputePassEncoder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, Default)]
pub struct ComputePassDescriptor<'a> {
    pub label: Option<&'a str>,
    pub timestamp_writes: Option<ComputeTimestampWrites<'a>>,
}

#[derive(Debug, Clone)]
pub struct ComputeTimestampWrites<'a> {
    pub query_set: &'a web_sys::GpuQuerySet,
    pub beginning_index: Option<u32>,
    pub end_index: Option<u32>,
}

// js conversions

impl From<ComputePassDescriptor<'_>> for web_sys::GpuComputePassDescriptor {
    fn from(pass: ComputePassDescriptor) -> web_sys::GpuComputePassDescriptor {
        let pass_js = web_sys::GpuComputePassDescriptor::new();

        if let Some(label) = pass.label {
            pass_js.set_label(label);
        }

        if let Some(timestamp_writes) = pass.timestamp_writes {
            pass_js.set_timestamp_writes(&web_sys::GpuComputePassTimestampWrites::from(
                timestamp_writes,
            ));
        }

        pass_js
    }
}

impl From<ComputeTimestampWrites<'_>> for web_sys::GpuComputePassTimestampWrites {
    fn from(timestamp_writes: ComputeTimestampWrites) -> web_sys::GpuComputePassTimestampWrites {
        let timestamp_writes_js =
            web_sys::GpuComputePassTimestampWrites::new(timestamp_writes.query_set);

        if let Some(beginning_index) = timestamp_writes.beginning_index {
            timestamp_writes_js.set_beginning_of_pass_write_index(beginning_index);
        }
        if let Some(end_index) = timestamp_writes.end_index {
            timestamp_writes_js.set_end_of_pass_write_index(end_index);
        }

        timestamp_writes_js
    }
}
