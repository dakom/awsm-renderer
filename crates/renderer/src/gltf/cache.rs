use std::{collections::HashMap, sync::Arc};

use crate::bind_groups::material::{MaterialKey, MaterialLayoutKey};

use super::{
    data::GltfData,
    materials::{GltfMaterialKey, GltfMaterialLayoutKey},
    pipelines::{GltfPipelineLayoutKey, GltfRenderPipelineKey},
};

#[derive(Default)]
pub(crate) struct GltfCache {
    pub render_pipelines: HashMap<GltfRenderPipelineKey, web_sys::GpuRenderPipeline>,
    pub pipeline_layouts: HashMap<GltfPipelineLayoutKey, web_sys::GpuPipelineLayout>,
    pub materials: HashMap<GltfMaterialKey, MaterialKey>,
    pub material_layouts: HashMap<GltfMaterialLayoutKey, MaterialLayoutKey>,
    pub raw_datas: Vec<Arc<GltfData>>,
}
