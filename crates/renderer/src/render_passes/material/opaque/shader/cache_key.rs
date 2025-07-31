use crate::{render_passes::{material::{cache_key::ShaderCacheKeyMaterial, looks::shader_cache_key::ShaderCacheKeyMaterialLook}, shader_cache_key::ShaderCacheKeyRenderPass}, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyOpaqueMaterial {
    pub look: ShaderCacheKeyMaterialLook
}


impl From<ShaderCacheKeyOpaqueMaterial> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyOpaqueMaterial) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Material(ShaderCacheKeyMaterial::Opaque(key)))
    }
}