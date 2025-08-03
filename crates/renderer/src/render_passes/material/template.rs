use askama::Template;

use crate::{render_passes::material::{cache_key::ShaderCacheKeyMaterial, opaque::shader::template::ShaderTemplateMaterialOpaque, transparent::shader::{cache_key::ShaderCacheKeyMaterialTransparent, template::ShaderTemplateMaterialTransparent}}, shaders::{AwsmShaderError, Result}};

#[derive(Debug)]
pub enum ShaderTemplateMaterial {
    Transparent(ShaderTemplateMaterialTransparent),
    Opaque(ShaderTemplateMaterialOpaque),
}

impl TryFrom<&ShaderCacheKeyMaterial> for ShaderTemplateMaterial {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyMaterial) -> Result<Self> {
        match value {
            ShaderCacheKeyMaterial::Opaque(cache_key) => Ok(ShaderTemplateMaterial::Opaque(cache_key.try_into()?)),
            ShaderCacheKeyMaterial::Transparent(cache_key) => Ok(ShaderTemplateMaterial::Transparent(cache_key.try_into()?)),
        }
    }
}

impl ShaderTemplateMaterial {
    pub fn into_source(self) -> Result<String> {
        match self {
            ShaderTemplateMaterial::Opaque(tmpl) => tmpl.into_source(),
            ShaderTemplateMaterial::Transparent(tmpl) => tmpl.into_source(),
        }
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        match self {
            ShaderTemplateMaterial::Opaque(tmpl) => tmpl.debug_label(),
            ShaderTemplateMaterial::Transparent(tmpl) => tmpl.debug_label(),
        }
    }
}

#[derive(Debug)]
pub struct ShaderTemplateVertexLocation {
    pub location: u32,
    pub interpolation: Option<&'static str>,
    pub name: String,
    pub data_type: String,
}

#[derive(Debug)]
pub struct ShaderTemplateVertexToFragmentAssignment {
    pub vertex_name: String,
    pub fragment_name: String,
}
