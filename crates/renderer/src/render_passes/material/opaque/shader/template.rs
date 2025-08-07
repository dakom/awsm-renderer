use askama::Template;

use crate::shaders::{AwsmShaderError, Result};

#[derive(Template, Debug)]
#[template(path = "material_opaque_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateMaterialOpaque {}

impl ShaderTemplateMaterialOpaque {
    pub fn into_source(self) -> Result<String> {
        Ok(self.render()?)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Opaque")
    }
}
