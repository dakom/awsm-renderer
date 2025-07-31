use crate::render_passes::material::looks::pbr::shader::cache_key::ShaderCacheKeyMaterialPbr;

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum ShaderCacheKeyMaterialLook {
    Pbr(ShaderCacheKeyMaterialPbr),
}