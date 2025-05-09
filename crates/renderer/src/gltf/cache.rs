use std::{collections::HashMap, sync::Arc};

use super::{
    data::GltfData,
    pipelines::{GltfPipelineLayoutKey, GltfRenderPipelineKey},
};

#[derive(Default)]
pub(crate) struct GltfCache {
    pub render_pipelines: HashMap<GltfRenderPipelineKey, web_sys::GpuRenderPipeline>,
    pub pipeline_layouts: HashMap<GltfPipelineLayoutKey, web_sys::GpuPipelineLayout>,
    pub raw_datas: Vec<Arc<GltfData>>,
}
