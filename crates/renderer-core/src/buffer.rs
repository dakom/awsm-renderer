#[derive(Debug, Clone)]
pub struct BufferBinding <'a> {
    pub buffer: &'a web_sys::GpuBuffer,
    pub offset: Option<f64>,
    pub size: Option<f64>,
}

impl <'a> BufferBinding <'a> {
    pub fn new(buffer: &'a web_sys::GpuBuffer) -> Self {
        Self { buffer, offset: None, size: None }
    }
}

impl From<BufferBinding<'_>> for web_sys::GpuBufferBinding {
    fn from(binding: BufferBinding) -> Self {
        let binding_js = web_sys::GpuBufferBinding::new(binding.buffer);

        if let Some(offset) = binding.offset {
            binding_js.set_offset(offset);
        }

        if let Some(size) = binding.size {
            binding_js.set_size(size);
        }

        binding_js
    }
}