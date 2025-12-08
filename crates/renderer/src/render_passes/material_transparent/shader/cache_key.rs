use crate::{
    render_passes::{
        shader_cache_key::ShaderCacheKeyRenderPass,
        shared::opaque_and_transparency::cache_key::ShaderMaterialVertexAttributes,
    },
    shaders::ShaderCacheKey,
};

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
