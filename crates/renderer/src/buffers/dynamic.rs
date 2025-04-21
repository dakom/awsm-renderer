use awsm_renderer_core::{bind_groups::BufferBindingType, buffers::BufferUsage};

#[derive(Debug, Clone)]
pub enum DynamicBufferKind {
    // uniform or storage
    Object {
        binding: u32,
        visibility_vertex: bool,
        visibility_fragment: bool,
        visibility_compute: bool,
        binding_type: BufferBindingType,
        usage: BufferUsage,
    },

    Vertex {
        usage: BufferUsage,
    },

    Index {
        usage: BufferUsage,
    },
}

impl DynamicBufferKind {
    pub fn new_uniform(binding: u32) -> Self {
        Self::Object {
            binding,
            visibility_vertex: true,
            visibility_fragment: false,
            visibility_compute: false,
            binding_type: BufferBindingType::Uniform,
            usage: BufferUsage::new().with_copy_dst().with_uniform(),
        }
    }

    pub fn new_storage(binding: u32, read_only: bool) -> Self {
        Self::Object {
            binding,
            visibility_vertex: true,
            visibility_fragment: false,
            visibility_compute: false,
            binding_type: if read_only {
                BufferBindingType::ReadOnlyStorage
            } else {
                BufferBindingType::Storage
            },
            usage: BufferUsage::new().with_copy_dst().with_storage(),
        }
    }

    pub fn new_vertex() -> Self {
        Self::Vertex {
            usage: BufferUsage::new().with_copy_dst().with_vertex(),
        }
    }

    pub fn new_index() -> Self {
        Self::Index {
            usage: BufferUsage::new().with_copy_dst().with_index(),
        }
    }

    pub fn usage(&self) -> BufferUsage {
        match self {
            Self::Object { usage, .. } => *usage,
            Self::Vertex { usage } => *usage,
            Self::Index { usage } => *usage,
        }
    }
}
