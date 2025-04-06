use std::{collections::HashMap, sync::Arc};

use super::{data::GltfData, pipelines::PipelineKey, shaders::ShaderKey};

#[derive(Default)]
pub struct GltfCache {
    pub shaders: HashMap<ShaderKey, web_sys::GpuShaderModule>,
    pub pipelines: HashMap<PipelineKey, web_sys::GpuRenderPipeline>,
    pub raw_datas: Vec<Arc<GltfData>>,
}
