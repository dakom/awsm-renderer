use askama::Template;

use crate::shaders::{fragment::{cache_key::ShaderCacheKeyFragment, entry::{debug_normals::ShaderTemplateFragmentDebugNormals, pbr::ShaderTemplateFragmentPbr, post_process::ShaderTemplateFragmentPostProcess}}, vertex::ShaderTemplateVertex};

// The struct that holds the shader template
#[derive(Debug)]
pub enum ShaderTemplateFragment {
    Pbr(ShaderTemplateFragmentPbr),
    PostProcess(ShaderTemplateFragmentPostProcess),
    DebugNormals(ShaderTemplateFragmentDebugNormals),
}

impl ShaderTemplateFragment {
    pub fn new(cache_key: &ShaderCacheKeyFragment, vertex: &mut ShaderTemplateVertex) -> Self {
        match cache_key {
            ShaderCacheKeyFragment::Pbr(cache_key) => {
                match vertex {
                    ShaderTemplateVertex::Mesh(mesh) => {
                        ShaderTemplateFragment::Pbr(ShaderTemplateFragmentPbr::new(cache_key, &mut mesh.vertex_to_fragment_assignments))
                    }
                    ShaderTemplateVertex::Quad(_) => {
                        ShaderTemplateFragment::Pbr(ShaderTemplateFragmentPbr::new(cache_key, &mut Vec::new()))
                    }
                }
            }
            ShaderCacheKeyFragment::PostProcess(cache_key) => {
                ShaderTemplateFragment::PostProcess(ShaderTemplateFragmentPostProcess::new(cache_key))
            }
            ShaderCacheKeyFragment::DebugNormals(cache_key) => {
                ShaderTemplateFragment::DebugNormals(ShaderTemplateFragmentDebugNormals::new(cache_key))
            }
        }
    }

    pub fn render(self) -> askama::Result<String> {
        match self {
            ShaderTemplateFragment::Pbr(pbr) => pbr.render(),
            ShaderTemplateFragment::PostProcess(post_process) => post_process.render(),
            ShaderTemplateFragment::DebugNormals(debug_normals) => debug_normals.render(),
        }
    }
}

#[derive(Debug)]
pub struct DynamicBufferBinding {
    pub group: u32,
    pub index: u32,
    pub name: String,
    pub data_type: String,
}


#[derive(Debug)]
pub struct ShaderTemplateFragmentLocation {
    pub location: u32,
    pub name: String,
    pub data_type: String,
}