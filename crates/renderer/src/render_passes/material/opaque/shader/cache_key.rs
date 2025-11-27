use crate::{
    mesh::{MeshBufferInfo, MeshBufferVertexAttributeInfo},
    render_passes::{
        material::cache_key::ShaderCacheKeyMaterial, shader_cache_key::ShaderCacheKeyRenderPass,
        shared::opaque_and_transparency::cache_key::ShaderMaterialVertexAttributes,
    },
    shaders::ShaderCacheKey,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyMaterialOpaque {
    pub attributes: ShaderMaterialVertexAttributes,
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
    pub msaa_sample_count: Option<u32>,
    pub mipmaps: bool,
}

impl From<ShaderCacheKeyMaterialOpaque> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyMaterialOpaque) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Material(
            ShaderCacheKeyMaterial::Opaque(key),
        ))
    }
}
