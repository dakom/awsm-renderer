use crate::{render_passes::{composite::shader::template::ShaderTemplateComposite, display::shader::template::ShaderTemplateDisplay, geometry::shader::template::ShaderTemplateGeometry, light_culling::shader::template::ShaderTemplateLightCulling, material::template::ShaderTemplateMaterial, shader_cache_key::ShaderCacheKeyRenderPass}, shaders::AwsmShaderError};


pub enum ShaderTemplateRenderPass {
    Geometry(ShaderTemplateGeometry),
    LightCulling(ShaderTemplateLightCulling),
    Material(ShaderTemplateMaterial),
    Composite(ShaderTemplateComposite),
    Display(ShaderTemplateDisplay),
}

impl TryFrom<&ShaderCacheKeyRenderPass> for ShaderTemplateRenderPass {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyRenderPass) -> std::result::Result<Self, Self::Error> {
        match value {
            ShaderCacheKeyRenderPass::Geometry(cache_key) => Ok(ShaderTemplateRenderPass::Geometry(cache_key.try_into()?)),
            ShaderCacheKeyRenderPass::LightCulling(cache_key) => Ok(ShaderTemplateRenderPass::LightCulling(cache_key.try_into()?)),
            ShaderCacheKeyRenderPass::Material(cache_key) => Ok(ShaderTemplateRenderPass::Material(cache_key.try_into()?)),
            ShaderCacheKeyRenderPass::Composite(cache_key) => Ok(ShaderTemplateRenderPass::Composite(cache_key.try_into()?)),
            ShaderCacheKeyRenderPass::Display(cache_key) => Ok(ShaderTemplateRenderPass::Display(cache_key.try_into()?)),
        }
    }
}

impl ShaderTemplateRenderPass {
    pub fn into_source(self) -> std::result::Result<String, AwsmShaderError> {
        match self {
            ShaderTemplateRenderPass::Geometry(tmpl) => tmpl.into_source(),
            ShaderTemplateRenderPass::LightCulling(tmpl) => tmpl.into_source(),
            ShaderTemplateRenderPass::Material(tmpl) => tmpl.into_source(),
            ShaderTemplateRenderPass::Composite(tmpl) => tmpl.into_source(),
            ShaderTemplateRenderPass::Display(tmpl) => tmpl.into_source(),
        }
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        match self {
            ShaderTemplateRenderPass::Geometry(tmpl) => tmpl.debug_label(),
            ShaderTemplateRenderPass::LightCulling(tmpl) => tmpl.debug_label(),
            ShaderTemplateRenderPass::Material(tmpl) => tmpl.debug_label(),
            ShaderTemplateRenderPass::Composite(tmpl) => tmpl.debug_label(),
            ShaderTemplateRenderPass::Display(tmpl) => tmpl.debug_label(),
        }
    }
}