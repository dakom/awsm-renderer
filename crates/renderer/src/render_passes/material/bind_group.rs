use crate::render_passes::material::{opaque::bind_group::MaterialOpaqueBindGroups, transparent::bind_group::MaterialTransparentBindGroups};

#[derive(Default)]
pub struct MaterialBindGroups {
    pub opaque: MaterialOpaqueBindGroups, 
    pub transparent: MaterialTransparentBindGroups, 
} 