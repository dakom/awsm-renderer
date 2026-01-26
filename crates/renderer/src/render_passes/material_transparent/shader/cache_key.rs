//! Shader cache key for the transparent material pass.

use crate::{
    render_passes::{
        shader_cache_key::ShaderCacheKeyRenderPass,
        shared::material::cache_key::ShaderMaterialVertexAttributes,
    },
    shaders::ShaderCacheKey,
};

/// Cache key for transparent material shaders.
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyMaterialTransparent {
    pub instancing_transforms: bool,
    pub attributes: ShaderMaterialVertexAttributes,
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
    pub msaa_sample_count: Option<u32>,
    pub mipmaps: bool,
}

impl From<ShaderCacheKeyMaterialTransparent> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyMaterialTransparent) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::MaterialTransparent(key))
    }
}
