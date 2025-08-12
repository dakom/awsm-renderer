use awsm_renderer_core::texture::mega_texture::MegaTextureBindings;

use crate::{
    render_passes::{
        material::cache_key::ShaderCacheKeyMaterial, shader_cache_key::ShaderCacheKeyRenderPass,
    },
    shaders::ShaderCacheKey,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyMaterialOpaque {
    pub texture_bindings: MegaTextureBindings,
}

impl From<ShaderCacheKeyMaterialOpaque> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyMaterialOpaque) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Material(
            ShaderCacheKeyMaterial::Opaque(key),
        ))
    }
}
