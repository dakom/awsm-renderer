use std::{collections::HashMap, sync::Arc};

use super::{buffers::BufferKey, loader::GltfResource, pipelines::PipelineKey, shaders::ShaderKey};

#[derive(Default)]
pub struct GltfCache {
    pub shaders: HashMap<ShaderKey, web_sys::GpuShaderModule>,
    pub pipelines: HashMap<PipelineKey, web_sys::GpuRenderPipeline>,
    // TODO - slotmap for gltf resources
    pub resources: Vec<Arc<GltfResource>>,
    pub buffers: HashMap<BufferKey, web_sys::GpuBuffer>,
}

// TODO - slotmap for gltf resources
pub type GltfResourceKey = usize;