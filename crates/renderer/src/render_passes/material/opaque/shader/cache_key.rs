use crate::{
    mesh::{MeshBufferInfo, MeshBufferVertexAttributeInfo},
    render_passes::{
        material::{
            cache_key::ShaderCacheKeyMaterial,
            opaque::shader::attributes::ShaderMaterialOpaqueVertexAttributes,
        },
        shader_cache_key::ShaderCacheKeyRenderPass,
    },
    shaders::ShaderCacheKey,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyMaterialOpaque {
    pub attributes: ShaderMaterialOpaqueVertexAttributes,
    pub texture_atlas_len: u32,
    pub sampler_atlas_len: u32,
    pub msaa_sample_count: u32, // 0 if no MSAA
    pub clamp_sampler_index: u32,
}

impl From<ShaderCacheKeyMaterialOpaque> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyMaterialOpaque) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Material(
            ShaderCacheKeyMaterial::Opaque(key),
        ))
    }
}
