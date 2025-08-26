use crate::{render_passes::shader_cache_key::ShaderCacheKeyRenderPass, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyGeometry {
    pub instancing_transforms: bool,
}

impl From<ShaderCacheKeyGeometry> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyGeometry) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Geometry(key))
    }
}
