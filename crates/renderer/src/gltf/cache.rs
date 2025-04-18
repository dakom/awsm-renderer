use std::{collections::HashMap, sync::Arc};

use crate::shaders::ShaderKey;

use super::{
    data::GltfData,
    pipelines::{PipelineLayoutKey, RenderPipelineKey},
};

#[derive(Default)]
pub(crate) struct GltfCache {
    pub shaders: HashMap<ShaderKey, web_sys::GpuShaderModule>,
    pub render_pipelines: HashMap<RenderPipelineKey, web_sys::GpuRenderPipeline>,
    pub pipeline_layouts: HashMap<PipelineLayoutKey, web_sys::GpuPipelineLayout>,
    pub raw_datas: Vec<Arc<GltfData>>,
}
