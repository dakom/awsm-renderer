use crate::{render_passes::shader_cache_key::ShaderCacheKeyRenderPass, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyDisplay {
}


impl From<ShaderCacheKeyDisplay> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyDisplay) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Display(key))
    }
}