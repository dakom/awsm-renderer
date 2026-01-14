use crate::{
    post_process::ToneMapping, render_passes::shader_cache_key::ShaderCacheKeyRenderPass,
    shaders::ShaderCacheKey,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyDisplay {
    pub smaa_anti_alias: bool,
    pub multisampled_geometry: bool,
    pub tonemapping: ToneMapping,
    pub bloom: bool,
    pub dof: bool,
}

impl From<ShaderCacheKeyDisplay> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyDisplay) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Display(key))
    }
}
