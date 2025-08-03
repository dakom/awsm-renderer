use crate::render_passes::{
    composite::shader::cache_key::ShaderCacheKeyComposite,
    display::shader::cache_key::ShaderCacheKeyDisplay,
    geometry::shader::cache_key::ShaderCacheKeyGeometry,
    light_culling::shader::cache_key::ShaderCacheKeyLightCulling,
    material::cache_key::ShaderCacheKeyMaterial,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum ShaderCacheKeyRenderPass {
    Geometry(ShaderCacheKeyGeometry),
    LightCulling(ShaderCacheKeyLightCulling),
    Material(ShaderCacheKeyMaterial),
    Composite(ShaderCacheKeyComposite),
    Display(ShaderCacheKeyDisplay),
}
