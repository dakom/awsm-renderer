const material_alignment = 256u; // must match `Materials::MAX_SIZE`

struct PbrMaterialRaw {
    // Basic properties, 14 * 4 = 56 bytes
    alpha_mode: u32,
    alpha_cutoff: f32,
    double_sided: u32,
    base_color_factor_r: f32,
    base_color_factor_g: f32,
    base_color_factor_b: f32,
    base_color_factor_a: f32,
    metallic_factor: f32,
    roughness_factor: f32,
    normal_scale: f32,
    occlusion_strength: f32,
    emissive_factor_r: f32,
    emissive_factor_g: f32,
    emissive_factor_b: f32,

    // Textures, 5 * 24 = 120 bytes
    base_color_tex_info: TextureInfoRaw,
    metallic_roughness_tex_info: TextureInfoRaw,
    normal_tex_info: TextureInfoRaw,
    occlusion_tex_info: TextureInfoRaw,
    emissive_tex_info: TextureInfoRaw,

    // this is set last, 4 bytes
    texture_bitmask: u32,

    // Padding to align to 256 bytes
    // 256 - (56 + 120 + 4) = 76 bytes padding, as 8 u32s
    padding: array<u32, 19>
};

struct PbrMaterial {
    alpha_mode: u32,
    alpha_cutoff: f32,

    double_sided: u32,

    has_base_color_texture: bool,
    base_color_tex_info: TextureInfo,
    base_color_factor: vec4<f32>,

    has_metallic_roughness_texture: bool,
    metallic_roughness_tex_info: TextureInfo,
    metallic_factor: f32,
    roughness_factor: f32,

    has_normal_texture: bool,
    normal_tex_info: TextureInfo,
    normal_scale: f32,

    has_occlusion_texture: bool,
    occlusion_tex_info: TextureInfo,
    occlusion_strength: f32,

    has_emissive_texture: bool,
    emissive_tex_info: TextureInfo,
    emissive_factor: vec3<f32>,
}

fn pbr_get_material(offset: u32) -> PbrMaterial {
    const TEXTURE_BITMASK_BASE_COLOR: u32 = 1u;
    const TEXTURE_BITMASK_METALIC_ROUGHNESS: u32 = 2u;
    const TEXTURE_BITMASK_NORMAL: u32 = 4u;
    const TEXTURE_BITMASK_OCCLUSION: u32 = 8u;
    const TEXTURE_BITMASK_EMISSIVE: u32 = 16u;

    let raw = materials[offset / material_alignment];

    return PbrMaterial(
        raw.alpha_mode,
        raw.alpha_cutoff,
        raw.double_sided,

        // base color
        (raw.texture_bitmask & TEXTURE_BITMASK_BASE_COLOR) != 0u,
        convert_texture_info(raw.base_color_tex_info),
        vec4<f32>(raw.base_color_factor_r, raw.base_color_factor_g, raw.base_color_factor_b, raw.base_color_factor_a),

        // metallic roughness
        (raw.texture_bitmask & TEXTURE_BITMASK_METALIC_ROUGHNESS) != 0u,
        convert_texture_info(raw.metallic_roughness_tex_info),
        raw.metallic_factor,
        raw.roughness_factor,

        // normal
        (raw.texture_bitmask & TEXTURE_BITMASK_NORMAL) != 0u,
        convert_texture_info(raw.normal_tex_info),
        raw.normal_scale,

        // occlusion
        (raw.texture_bitmask & TEXTURE_BITMASK_OCCLUSION) != 0u,
        convert_texture_info(raw.occlusion_tex_info),
        raw.occlusion_strength,

        // emissive
        (raw.texture_bitmask & TEXTURE_BITMASK_EMISSIVE) != 0u,
        convert_texture_info(raw.emissive_tex_info),
        vec3<f32>(raw.emissive_factor_r, raw.emissive_factor_g, raw.emissive_factor_b),
    );
}
