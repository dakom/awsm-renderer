use crate::{
    render_passes::{
        material::transparent::shader::cache_key::ShaderCacheKeyMaterialTransparent,
        shader_cache_key::ShaderCacheKeyRenderPass,
    },
    shaders::ShaderCacheKey,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum ShaderCacheKeyMaterial {
    Opaque,
    Transparent(ShaderCacheKeyMaterialTransparent),
}

impl From<ShaderCacheKeyMaterial> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyMaterial) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Material(key))
    }
}
