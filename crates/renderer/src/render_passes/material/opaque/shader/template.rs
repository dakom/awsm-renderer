use askama::Template;

use crate::{render_passes::material::opaque::shader::cache_key::ShaderCacheKeyMaterialOpaque, shaders::{Result, AwsmShaderError}};

#[derive(Template, Debug)]
#[template(path = "material_opaque_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateMaterialOpaque {
}

impl TryFrom<&ShaderCacheKeyMaterialOpaque> for ShaderTemplateMaterialOpaque {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyMaterialOpaque) -> Result<Self> {
        Ok(Self {
        })
    }
}

impl ShaderTemplateMaterialOpaque{
    pub fn into_source(self) -> Result<String> {
        Ok(self.render()?)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Opaque")
    }
}