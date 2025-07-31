use askama::Template;

use crate::{render_passes::material::opaque::shader::cache_key::ShaderCacheKeyOpaqueMaterial, shaders::{Result, AwsmShaderError}};

#[derive(Template, Debug)]
#[template(path = "material_opaque_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateOpaqueMaterial {
}

impl TryFrom<&ShaderCacheKeyOpaqueMaterial> for ShaderTemplateOpaqueMaterial {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyOpaqueMaterial) -> Result<Self> {
        Ok(Self {
        })
    }
}

impl ShaderTemplateOpaqueMaterial {
    pub fn into_source(self) -> Result<String> {
        Ok(self.render()?)
    }
}