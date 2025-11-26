use askama::Template;

use crate::{
    render_passes::material::transparent::shader::cache_key::ShaderCacheKeyMaterialTransparent,
    shaders::{AwsmShaderError, Result},
};

#[derive(Debug)]
pub struct ShaderTemplateMaterialTransparent {
    pub vertex: ShaderTemplateTransparentMaterialVertex,
    pub fragment: ShaderTemplateTransparentMaterialFragment,
}

#[derive(Template, Debug)]
#[template(
    path = "material_transparent_wgsl/vertex.wgsl",
    whitespace = "minimize"
)]
pub struct ShaderTemplateTransparentMaterialVertex {
    max_morph_unroll: u32,
    max_skin_unroll: u32,
    instancing_transforms: bool,
}

impl ShaderTemplateTransparentMaterialVertex {
    pub fn new(cache_key: &ShaderCacheKeyMaterialTransparent) -> Self {
        Self {
            max_morph_unroll: 2,
            max_skin_unroll: 2,
            instancing_transforms: cache_key.instancing_transforms,
        }
    }
}

#[derive(Template, Debug)]
#[template(
    path = "material_transparent_wgsl/fragment.wgsl",
    whitespace = "minimize"
)]
pub struct ShaderTemplateTransparentMaterialFragment {}

impl ShaderTemplateTransparentMaterialFragment {
    pub fn new(_cache_key: &ShaderCacheKeyMaterialTransparent) -> Self {
        Self {}
    }
}

impl TryFrom<&ShaderCacheKeyMaterialTransparent> for ShaderTemplateMaterialTransparent {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyMaterialTransparent) -> Result<Self> {
        Ok(Self {
            vertex: ShaderTemplateTransparentMaterialVertex::new(value),
            fragment: ShaderTemplateTransparentMaterialFragment::new(value),
        })
    }
}

impl ShaderTemplateMaterialTransparent {
    pub fn into_source(self) -> Result<String> {
        let vertex_source = self.vertex.render()?;
        let fragment_source = self.fragment.render()?;
        Ok(format!("{}\n{}", vertex_source, fragment_source))
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Transparent")
    }
}
