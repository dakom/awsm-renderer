#[derive(Debug, Clone)]
pub struct BufferDescriptor<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBuffer#descriptor
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuBufferDescriptor.html
    pub label: Option<&'a str>,
    pub mapped_at_creation: Option<bool>,
    pub size: u64,
    pub usage: BufferUsage,
}

#[derive(Debug, Clone)]
pub struct BufferBinding<'a> {
    pub buffer: &'a web_sys::GpuBuffer,
    pub offset: Option<f64>,
    pub size: Option<f64>,
}

impl<'a> BufferBinding<'a> {
    pub fn new(buffer: &'a web_sys::GpuBuffer) -> Self {
        Self {
            buffer,
            offset: None,
            size: None,
        }
    }
}


#[derive(Debug, Clone, Default)]
pub struct BufferUsage {
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/gpu_buffer_usage/index.html
    pub copy_dst: bool,
    pub copy_src: bool,
    pub index: bool,
    pub indirect: bool,
    pub map_read: bool,
    pub map_write: bool,
    pub query_resolve: bool,
    pub storage: bool,
    pub uniform: bool,
    pub vertex: bool,
}

impl BufferUsage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_u32(&self) -> u32 {
        let mut usage = 0;
        if self.copy_dst {
            usage |= web_sys::gpu_buffer_usage::COPY_DST;
        }
        if self.copy_src {
            usage |= web_sys::gpu_buffer_usage::COPY_SRC;
        }
        if self.index {
            usage |= web_sys::gpu_buffer_usage::INDEX;
        }
        if self.indirect {
            usage |= web_sys::gpu_buffer_usage::INDIRECT;
        }
        if self.map_read {
            usage |= web_sys::gpu_buffer_usage::MAP_READ;
        }
        if self.map_write {
            usage |= web_sys::gpu_buffer_usage::MAP_WRITE;
        }
        if self.query_resolve {
            usage |= web_sys::gpu_buffer_usage::QUERY_RESOLVE;
        }
        if self.storage {
            usage |= web_sys::gpu_buffer_usage::STORAGE;
        }
        if self.uniform {
            usage |= web_sys::gpu_buffer_usage::UNIFORM;
        }
        if self.vertex {
            usage |= web_sys::gpu_buffer_usage::VERTEX;
        }
        usage
    }

    pub fn with_copy_dst(mut self) -> Self {
        self.copy_dst = true;
        self
    }

    pub fn with_copy_src(mut self) -> Self {
        self.copy_src = true;
        self
    }

    pub fn with_index(mut self) -> Self {
        self.index = true;
        self
    }

    pub fn with_indirect(mut self) -> Self {
        self.indirect = true;
        self
    }

    pub fn with_map_read(mut self) -> Self {
        self.map_read = true;
        self
    }

    pub fn with_map_write(mut self) -> Self {
        self.map_write = true;
        self
    }

    pub fn with_query_resolve(mut self) -> Self {
        self.query_resolve = true;
        self
    }

    pub fn with_storage(mut self) -> Self {
        self.storage = true;
        self
    }

    pub fn with_uniform(mut self) -> Self {
        self.uniform = true;
        self
    }

    pub fn with_vertex(mut self) -> Self {
        self.vertex = true;
        self
    }
}

// js conversion

impl From<BufferDescriptor<'_>> for web_sys::GpuBufferDescriptor {
    fn from(descriptor: BufferDescriptor) -> Self {
        let descriptor_js = web_sys::GpuBufferDescriptor::new(descriptor.size as f64, descriptor.usage.as_u32());

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
            binding_js.set_offset(offset);
        }

        if let Some(size) = binding.size {
            binding_js.set_size(size);
        }

        binding_js
    }
}