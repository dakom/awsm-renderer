use askama::Template;

use crate::{render_passes::light_culling::shader::cache_key::ShaderCacheKeyLightCulling, shaders::{Result, AwsmShaderError}};


#[derive(Template, Debug)]
#[template(path = "light_culling_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateLightCulling {
}

impl TryFrom<&ShaderCacheKeyLightCulling> for ShaderTemplateLightCulling {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyLightCulling) -> Result<Self> {
        Ok(Self {
        })
    }
}

impl ShaderTemplateLightCulling {
    pub fn into_source(self) -> Result<String> {
        Ok(self.render()?)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Light Culling")
    }
}