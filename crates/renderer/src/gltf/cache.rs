use std::collections::HashMap;

use super::{pipelines::PipelineKey, shaders::ShaderKey};

#[derive(Default)]
pub struct GltfCache {
    pub shaders: HashMap<ShaderKey, web_sys::GpuShaderModule>,
    pub pipelines: HashMap<PipelineKey, web_sys::GpuRenderPipeline>,
}
