use askama::Template;

use crate::shaders::fragment::entry::debug_normals::ShaderCacheKeyFragmentDebugNormals;

#[derive(Template, Debug)]
#[template(path = "fragment/debug_normals.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateFragmentDebugNormals {
    pub has_normals: bool,
}

impl ShaderTemplateFragmentDebugNormals {
    pub fn new(cache_key: &ShaderCacheKeyFragmentDebugNormals) -> Self {
        Self { has_normals: cache_key.has_normals }
    }
}