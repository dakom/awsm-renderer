use crate::{
    render_passes::{
        composite::shader::template::ShaderTemplateComposite,
        display::shader::template::ShaderTemplateDisplay,
        geometry::shader::template::ShaderTemplateGeometry,
        light_culling::shader::template::ShaderTemplateLightCulling,
        material_opaque::shader::{
            cache_key::ShaderCacheKeyMaterialOpaque, template::ShaderTemplateMaterialOpaque,
        },
        material_transparent::shader::{
            cache_key::ShaderCacheKeyMaterialTransparent,
            template::ShaderTemplateMaterialTransparent,
        },
        shader_cache_key::ShaderCacheKeyRenderPass,
    },
    shaders::AwsmShaderError,
};

pub enum ShaderTemplateRenderPass {
    Geometry(ShaderTemplateGeometry),
    LightCulling(ShaderTemplateLightCulling),
    MaterialOpaque(ShaderTemplateMaterialOpaque),
    MaterialTransparent(ShaderTemplateMaterialTransparent),
    Composite(ShaderTemplateComposite),
    Display(ShaderTemplateDisplay),
}

impl TryFrom<&ShaderCacheKeyRenderPass> for ShaderTemplateRenderPass {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyRenderPass) -> std::result::Result<Self, Self::Error> {
        match value {
            ShaderCacheKeyRenderPass::Geometry(cache_key) => {
                Ok(ShaderTemplateRenderPass::Geometry(cache_key.try_into()?))
            }
            ShaderCacheKeyRenderPass::LightCulling(cache_key) => Ok(
                ShaderTemplateRenderPass::LightCulling(cache_key.try_into()?),
            ),
            ShaderCacheKeyRenderPass::MaterialOpaque(cache_key) => Ok(
                ShaderTemplateRenderPass::MaterialOpaque(cache_key.try_into()?),
            ),
            ShaderCacheKeyRenderPass::MaterialTransparent(cache_key) => Ok(
                ShaderTemplateRenderPass::MaterialTransparent(cache_key.try_into()?),
            ),
            ShaderCacheKeyRenderPass::Composite(cache_key) => {
                Ok(ShaderTemplateRenderPass::Composite(cache_key.try_into()?))
            }
            ShaderCacheKeyRenderPass::Display(cache_key) => {
                Ok(ShaderTemplateRenderPass::Display(cache_key.try_into()?))
            }
        }
    }
}

impl ShaderTemplateRenderPass {
    pub fn into_source(self) -> std::result::Result<String, AwsmShaderError> {
        match self {
            ShaderTemplateRenderPass::Geometry(tmpl) => tmpl.into_source(),
            ShaderTemplateRenderPass::LightCulling(tmpl) => tmpl.into_source(),
            ShaderTemplateRenderPass::MaterialOpaque(tmpl) => tmpl.into_source(),
            ShaderTemplateRenderPass::MaterialTransparent(tmpl) => tmpl.into_source(),
            ShaderTemplateRenderPass::Composite(tmpl) => tmpl.into_source(),
            ShaderTemplateRenderPass::Display(tmpl) => tmpl.into_source(),
        }
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        match self {
            ShaderTemplateRenderPass::Geometry(tmpl) => tmpl.debug_label(),
            ShaderTemplateRenderPass::LightCulling(tmpl) => tmpl.debug_label(),
            ShaderTemplateRenderPass::MaterialOpaque(tmpl) => tmpl.debug_label(),
            ShaderTemplateRenderPass::MaterialTransparent(tmpl) => tmpl.debug_label(),
            ShaderTemplateRenderPass::Composite(tmpl) => tmpl.debug_label(),
            ShaderTemplateRenderPass::Display(tmpl) => tmpl.debug_label(),
        }
    }
}
