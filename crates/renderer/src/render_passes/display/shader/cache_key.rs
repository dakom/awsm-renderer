use crate::{
    post_process::ToneMapping, render_passes::shader_cache_key::ShaderCacheKeyRenderPass,
    shaders::ShaderCacheKey,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyDisplay {
    pub smaa_anti_alias: bool,
    pub tonemapping: ToneMapping,
}

impl From<ShaderCacheKeyDisplay> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyDisplay) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Display(key))
    }
}
