use askama::Template;

use crate::{render_passes::material::transparent::shader::cache_key::ShaderCacheKeyTransparentMaterial, shaders::{Result, AwsmShaderError}};

#[derive(Debug)]
pub struct ShaderTemplateTransparentMaterial {
    pub vertex: ShaderTemplateTransparentMaterialVertex,
    pub fragment: ShaderTemplateTransparentMaterialFragment,
}

#[derive(Template, Debug)]
#[template(path = "material_transparent_wgsl/vertex.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateTransparentMaterialVertex {
}

#[derive(Template, Debug)]
#[template(path = "material_transparent_wgsl/fragment.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateTransparentMaterialFragment {
}

impl TryFrom<&ShaderCacheKeyTransparentMaterial> for ShaderTemplateTransparentMaterial {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyTransparentMaterial) -> Result<Self> {
        Ok(Self {
            vertex: ShaderTemplateTransparentMaterialVertex {},
            fragment: ShaderTemplateTransparentMaterialFragment {},
        })
    }
}

impl ShaderTemplateTransparentMaterial {
    pub fn into_source(self) -> Result<String> {
        let vertex_source = self.vertex.render()?;
        let fragment_source = self.fragment.render()?;
        Ok(format!("{}\n{}", vertex_source, fragment_source))
    }
}