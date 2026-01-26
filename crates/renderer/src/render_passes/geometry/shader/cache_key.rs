//! Shader cache key for the geometry pass.

use crate::{render_passes::shader_cache_key::ShaderCacheKeyRenderPass, shaders::ShaderCacheKey};

/// Cache key for geometry pass shaders.
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyGeometry {
    pub instancing_transforms: bool,
    pub msaa_samples: Option<u32>,
}

impl From<ShaderCacheKeyGeometry> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyGeometry) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Geometry(key))
    }
}
