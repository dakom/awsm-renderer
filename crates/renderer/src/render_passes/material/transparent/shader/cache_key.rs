use crate::{render_passes::{material::{cache_key::ShaderCacheKeyMaterial, looks::shader_cache_key::ShaderCacheKeyMaterialLook}, shader_cache_key::ShaderCacheKeyRenderPass}, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyMaterialTransparent {
    pub look: ShaderCacheKeyMaterialLook
}


impl From<ShaderCacheKeyMaterialTransparent> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyMaterialTransparent) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Material(ShaderCacheKeyMaterial::Transparent(key)))
    }
}