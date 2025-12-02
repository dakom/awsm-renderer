use crate::render_passes::{
    composite::shader::cache_key::ShaderCacheKeyComposite,
    display::shader::cache_key::ShaderCacheKeyDisplay,
    geometry::shader::cache_key::ShaderCacheKeyGeometry,
    light_culling::shader::cache_key::ShaderCacheKeyLightCulling,
    material_opaque::shader::cache_key::ShaderCacheKeyMaterialOpaque,
    material_transparent::shader::cache_key::ShaderCacheKeyMaterialTransparent,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum ShaderCacheKeyRenderPass {
    Geometry(ShaderCacheKeyGeometry),
    LightCulling(ShaderCacheKeyLightCulling),
    MaterialOpaque(ShaderCacheKeyMaterialOpaque),
    MaterialTransparent(ShaderCacheKeyMaterialTransparent),
    Composite(ShaderCacheKeyComposite),
    Display(ShaderCacheKeyDisplay),
}
