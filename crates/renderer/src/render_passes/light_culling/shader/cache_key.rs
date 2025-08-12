use crate::{render_passes::shader_cache_key::ShaderCacheKeyRenderPass, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyLightCulling {}

impl From<ShaderCacheKeyLightCulling> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyLightCulling) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::LightCulling(key))
    }
}
