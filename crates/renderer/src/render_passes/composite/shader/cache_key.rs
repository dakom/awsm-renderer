use crate::{render_passes::shader_cache_key::ShaderCacheKeyRenderPass, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyComposite {
    pub multisampled_geometry: bool,
}

impl From<ShaderCacheKeyComposite> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyComposite) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Composite(key))
    }
}
