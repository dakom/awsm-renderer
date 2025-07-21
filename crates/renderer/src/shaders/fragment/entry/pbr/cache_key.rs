#[derive(Default, Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderCacheKeyFragmentPbr {
    pub base_color_uv_index: Option<u32>,
    pub metallic_roughness_uv_index: Option<u32>,
    pub normal_uv_index: Option<u32>,
    pub occlusion_uv_index: Option<u32>,
    pub emissive_uv_index: Option<u32>,
    pub has_alpha_mask: bool,
    pub has_normals: bool, // actually comes from vertex shader, but affects fragment shader
}
