use crate::{render_passes::{material::{cache_key::ShaderCacheKeyMaterial, looks::shader_cache_key::ShaderCacheKeyMaterialLook}, shader_cache_key::ShaderCacheKeyRenderPass}, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyMaterialOpaque {
    pub look: ShaderCacheKeyMaterialLook
}


impl From<ShaderCacheKeyMaterialOpaque> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyMaterialOpaque) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Material(ShaderCacheKeyMaterial::Opaque(key)))
    }
}