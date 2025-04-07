use std::{collections::HashMap, sync::Arc};

use super::{data::GltfData, pipelines::{RenderPipelineKey, PipelineLayoutKey}, shaders::ShaderKey};

#[derive(Default)]
pub struct GltfCache {
    pub shaders: HashMap<ShaderKey, web_sys::GpuShaderModule>,
    pub render_pipelines: HashMap<RenderPipelineKey, web_sys::GpuRenderPipeline>,
    pub pipeline_layouts: HashMap<PipelineLayoutKey, web_sys::GpuPipelineLayout>,
    pub raw_datas: Vec<Arc<GltfData>>,
}
