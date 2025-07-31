use crate::{render_passes::{material::{cache_key::ShaderCacheKeyMaterial, looks::shader_cache_key::ShaderCacheKeyMaterialLook}, shader_cache_key::ShaderCacheKeyRenderPass}, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyTransparentMaterial {
    pub look: ShaderCacheKeyMaterialLook
}


impl From<ShaderCacheKeyTransparentMaterial> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyTransparentMaterial) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Material(ShaderCacheKeyMaterial::Transparent(key)))
    }
}