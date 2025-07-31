use crate::{render_passes::{material::{opaque::shader::cache_key::ShaderCacheKeyOpaqueMaterial, transparent::shader::cache_key::ShaderCacheKeyTransparentMaterial}, shader_cache_key::ShaderCacheKeyRenderPass}, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum ShaderCacheKeyMaterial {
    Opaque(ShaderCacheKeyOpaqueMaterial),
    Transparent(ShaderCacheKeyTransparentMaterial),
}


impl From<ShaderCacheKeyMaterial> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyMaterial) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Material(key))
    }
}