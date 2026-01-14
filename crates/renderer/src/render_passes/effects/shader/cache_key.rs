use crate::{render_passes::shader_cache_key::ShaderCacheKeyRenderPass, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyEffects {
    pub smaa_anti_alias: bool,
    pub multisampled_geometry: bool,
    pub bloom: bool,
    pub dof: bool,
}

impl From<ShaderCacheKeyEffects> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyEffects) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Effects(key))
    }
}
