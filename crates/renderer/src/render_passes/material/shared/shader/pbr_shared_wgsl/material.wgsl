const material_alignment = 512u; // must match `Materials::MAX_SIZE`

// This must match PbrMaterial in Rust
// and the size is PbrMaterial::BYTE_SIZE
struct PbrMaterialRaw {
    // Basic properties, 15 * 4 = 60 bytes
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
    emissive_strength: f32,

    // Textures, 5 * 64 = 320 bytes (added UV transform data)
    base_color_tex_info: TextureInfoRaw,
    metallic_roughness_tex_info: TextureInfoRaw,
    normal_tex_info: TextureInfoRaw,
    occlusion_tex_info: TextureInfoRaw,
    emissive_tex_info: TextureInfoRaw,

    // Color info, 4 bytes
    color_info: ColorInfo,

    // this is set last, 4 bytes
    bitmask: u32,

    // Padding to align to 512 bytes (60 + 280 + 4 + 4 = 348, so 512 - 348 = 164 bytes = 41 u32s)
    padding: array<u32, 31>
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
    emissive_strength: f32,

    has_color_info: bool,
    color_info: ColorInfo,
}

fn pbr_get_material(offset: u32) -> PbrMaterial {
    const MATERIAL_BITMASK_BASE_COLOR: u32 = 1u;
    const MATERIAL_BITMASK_METALIC_ROUGHNESS: u32 = 1u << 1u;;
    const MATERIAL_BITMASK_NORMAL: u32 = 1u << 2u;;
    const MATERIAL_BITMASK_OCCLUSION: u32 = 1u << 3u;;
    const MATERIAL_BITMASK_EMISSIVE: u32 = 1u << 4u;;
    const MATERIAL_BITMASK_COLOR: u32 = 1u << 5u;;

    let raw = materials[offset / material_alignment];

    return PbrMaterial(
        raw.alpha_mode,
        raw.alpha_cutoff,
        raw.double_sided,

        // base color
        (raw.bitmask & MATERIAL_BITMASK_BASE_COLOR) != 0u,
        convert_texture_info(raw.base_color_tex_info),
        vec4<f32>(raw.base_color_factor_r, raw.base_color_factor_g, raw.base_color_factor_b, raw.base_color_factor_a),

        // metallic roughness
        (raw.bitmask & MATERIAL_BITMASK_METALIC_ROUGHNESS) != 0u,
        convert_texture_info(raw.metallic_roughness_tex_info),
        raw.metallic_factor,
        raw.roughness_factor,

        // normal
        (raw.bitmask & MATERIAL_BITMASK_NORMAL) != 0u,
        convert_texture_info(raw.normal_tex_info),
        raw.normal_scale,

        // occlusion
        (raw.bitmask & MATERIAL_BITMASK_OCCLUSION) != 0u,
        convert_texture_info(raw.occlusion_tex_info),
        raw.occlusion_strength,

        // emissive
        (raw.bitmask & MATERIAL_BITMASK_EMISSIVE) != 0u,
        convert_texture_info(raw.emissive_tex_info),
        vec3<f32>(raw.emissive_factor_r, raw.emissive_factor_g, raw.emissive_factor_b),
        raw.emissive_strength,

        // color
        (raw.bitmask & MATERIAL_BITMASK_COLOR) != 0u,
        raw.color_info
    );
}
