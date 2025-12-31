use wasm_bindgen_futures::JsFuture;

use crate::error::AwsmCoreError;

#[derive(Debug, Clone)]
pub struct BufferDescriptor<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBuffer#descriptor
    // https://docs.rs/web-sys/latest/web_sys/struct.GpuBufferDescriptor.html
    pub label: Option<&'a str>,
    pub mapped_at_creation: Option<bool>,
    pub size: usize,
    pub usage: BufferUsage,
}

impl<'a> BufferDescriptor<'a> {
    pub fn new(label: Option<&'a str>, size: usize, usage: BufferUsage) -> Self {
        Self {
            label,
            size,
            usage,
            mapped_at_creation: None,
        }
    }

    pub fn with_mapped_at_creation(mut self, mapped_at_creation: bool) -> Self {
        self.mapped_at_creation = Some(mapped_at_creation);
        self
    }
}

#[derive(Debug, Clone)]
pub struct BufferBinding<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBindGroup#gpubufferbinding_objects
    // https://docs.rs/web-sys/latest/web_sys/struct.GpuBufferBinding.html
    pub buffer: &'a web_sys::GpuBuffer,
    pub offset: Option<usize>,
    pub size: Option<usize>,
}

impl<'a> BufferBinding<'a> {
    pub fn new(buffer: &'a web_sys::GpuBuffer) -> Self {
        Self {
            buffer,
            offset: None,
            size: None,
        }
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.size = Some(size);
        self
    }
}

#[derive(Hash, Debug, Clone, Default, Copy, PartialEq, Eq)]
// https://docs.rs/web-sys/latest/web_sys/gpu_buffer_usage/index.html
pub struct BufferUsage(u32);

impl From<u32> for BufferUsage {
    fn from(usage: u32) -> Self {
        Self(usage)
    }
}
impl From<BufferUsage> for u32 {
    fn from(usage: BufferUsage) -> Self {
        usage.0
    }
}

impl BufferUsage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_copy_dst(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::COPY_DST;
        self
    }

    pub fn with_copy_src(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::COPY_SRC;
        self
    }

    pub fn with_index(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::INDEX;
        self
    }

    pub fn with_indirect(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::INDIRECT;
        self
    }

    pub fn with_map_read(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::MAP_READ;
        self
    }

    pub fn with_map_write(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::MAP_WRITE;
        self
    }

    pub fn with_query_resolve(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::QUERY_RESOLVE;
        self
    }

    pub fn with_storage(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::STORAGE;
        self
    }

    pub fn with_uniform(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::UNIFORM;
        self
    }

    pub fn with_vertex(mut self) -> Self {
        self.0 |= web_sys::gpu_buffer_usage::VERTEX;
        self
    }
}

// https://docs.rs/web-sys/latest/src/web_sys/features/gen_gpu_map_mode.rs.html#5
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MapMode {
    Read = web_sys::gpu_map_mode::READ,
    Write = web_sys::gpu_map_mode::WRITE,
}

/// Extracts GPU buffer data into a new mapped buffer and returns it as a `Vec<u8>`
pub async fn extract_buffer_vec(
    read_buffer: &web_sys::GpuBuffer,
    size: Option<u32>,
) -> crate::error::Result<Vec<u8>> {
    let size = size.unwrap_or(read_buffer.size() as u32);

    // Wait for GPU to complete mapping
    let map_promise = read_buffer.map_async_with_u32_and_u32(MapMode::Read as u32, 0, size);
    JsFuture::from(map_promise)
        .await
        .map_err(AwsmCoreError::buffer_map)?;

    // Get the mapped JS ArrayBuffer slice
    let array_buffer = read_buffer
        .get_mapped_range_with_u32_and_u32(0, size)
        .map_err(AwsmCoreError::buffer_map_range)?;

    // Convert to Uint8Array
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    let mut vec = vec![0u8; size as usize];
    uint8_array.copy_to(&mut vec);

    read_buffer.unmap();

    Ok(vec)
}

pub async fn extract_buffer_array<const N: usize>(
    read_buffer: &web_sys::GpuBuffer,
    dest: &mut [u8; N],
) -> crate::error::Result<()> {
    // Wait for GPU to complete mapping
    let map_promise = read_buffer.map_async_with_u32_and_u32(MapMode::Read as u32, 0, N as u32);
    JsFuture::from(map_promise)
        .await
        .map_err(AwsmCoreError::buffer_map)?;

    // Get the mapped JS ArrayBuffer slice
    let array_buffer = read_buffer
        .get_mapped_range_with_u32_and_u32(0, N as u32)
        .map_err(AwsmCoreError::buffer_map_range)?;

    // Convert to Uint8Array
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    uint8_array.copy_to(dest);

    read_buffer.unmap();

    Ok(())
}

// js conversion

impl From<BufferDescriptor<'_>> for web_sys::GpuBufferDescriptor {
    fn from(descriptor: BufferDescriptor) -> Self {
        let descriptor_js =
            web_sys::GpuBufferDescriptor::new(descriptor.size as f64, descriptor.usage.into());

        if let Some(label) = descriptor.label {
            descriptor_js.set_label(label);
        }

        if let Some(mapped_at_creation) = descriptor.mapped_at_creation {
            descriptor_js.set_mapped_at_creation(mapped_at_creation);
        }

        descriptor_js
    }
}

impl From<BufferBinding<'_>> for web_sys::GpuBufferBinding {
    fn from(binding: BufferBinding) -> Self {
        let binding_js = web_sys::GpuBufferBinding::new(binding.buffer);

        if let Some(offset) = binding.offset {
            binding_js.set_offset(offset as f64);
        }

        if let Some(size) = binding.size {
            binding_js.set_size(size as f64);
        }

        binding_js
    }
}
