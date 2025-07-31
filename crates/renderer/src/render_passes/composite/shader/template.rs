use askama::Template;

use crate::{render_passes::{composite::shader::cache_key::ShaderCacheKeyComposite, material::opaque::shader::cache_key::ShaderCacheKeyOpaqueMaterial}, shaders::{AwsmShaderError, Result}};

#[derive(Template, Debug)]
#[template(path = "composite_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateComposite {
}

impl TryFrom<&ShaderCacheKeyComposite> for ShaderTemplateComposite {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyComposite) -> Result<Self> {
        Ok(Self {
        })
    }
}

impl ShaderTemplateComposite {
    pub fn into_source(self) -> Result<String> {
        Ok(self.render()?)
    }
}