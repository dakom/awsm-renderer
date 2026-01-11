const material_alignment = 512u; // must match `Materials::MAX_SIZE`

// must match MaterialAlphaMode::variant_as_u32
const ALPHA_MODE_OPAQUE: u32 = 0u;
const ALPHA_MODE_MASK: u32 = 1u;
const ALPHA_MODE_BLEND: u32 = 2u;

// This must match PbrMaterial in Rust
// and the size is PbrMaterial::BYTE_SIZE
struct PbrMaterialRaw {
    // Basic properties, 26 * 4 = 104 bytes
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
    specular: f32,
    specular_color_r: f32,
    specular_color_g: f32,
    specular_color_b: f32,
    ior: f32,
    transmission_factor: f32,
    volume_thickness_factor: f32,
    volume_attenuation_distance: f32,
    volume_attenuation_color_r: f32,
    volume_attenuation_color_g: f32,
    volume_attenuation_color_b: f32,


    // Textures, 9 * 20 = 180 bytes (packed format)
    base_color_tex_info: TextureInfoRaw,
    metallic_roughness_tex_info: TextureInfoRaw,
    normal_tex_info: TextureInfoRaw,
    occlusion_tex_info: TextureInfoRaw,
    emissive_tex_info: TextureInfoRaw,
    specular_tex_info: TextureInfoRaw,
    specular_color_tex_info: TextureInfoRaw,
    transmission_tex_info: TextureInfoRaw,
    volume_thickness_tex_info: TextureInfoRaw,

    // Color info, 4 bytes
    color_info: ColorInfo,

    // this is set last, 4 bytes
    bitmask: u32,

    // Padding to align to 512 bytes (104 + 180 + 4 + 4 = 292, so 512 - 292 = 220 bytes = 55 u32s)
    padding: array<u32, 55>
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

    has_specular_texture: bool,
    specular_tex_info: TextureInfo,
    specular_factor: f32,

    has_specular_color_texture: bool,
    specular_color_tex_info: TextureInfo,
    specular_color_factor: vec3<f32>,

    ior: f32,

    has_transmission_texture: bool,
    transmission_tex_info: TextureInfo,
    transmission_factor: f32,

    has_volume_thickness_texture: bool,
    volume_thickness_tex_info: TextureInfo,
    volume_thickness_factor: f32,
    volume_attenuation_distance: f32,
    volume_attenuation_color: vec3<f32>,
}

fn pbr_get_material(offset: u32) -> PbrMaterial {
    // must correspond to material.rs
    const BITMASK_BASE_COLOR: u32 = 1u;
    const BITMASK_METALIC_ROUGHNESS: u32 = 1u << 1u;
    const BITMASK_NORMAL: u32 = 1u << 2u;
    const BITMASK_OCCLUSION: u32 = 1u << 3u;
    const BITMASK_EMISSIVE: u32 = 1u << 4u;
    const BITMASK_COLOR: u32 = 1u << 5u;
    const BITMASK_SPECULAR: u32 = 1u << 6u;
    const BITMASK_SPECULAR_COLOR: u32 = 1u << 7u;
    const BITMASK_TRANSMISSION : u32 = 1u << 8u;
    const BITMASK_VOLUME_THICKNESS : u32 = 1u << 9u;

    let raw = materials[offset / material_alignment];

    return PbrMaterial(
        raw.alpha_mode,
        raw.alpha_cutoff,
        raw.double_sided,

        // base color
        (raw.bitmask & BITMASK_BASE_COLOR) != 0u,
        convert_texture_info(raw.base_color_tex_info),
        vec4<f32>(raw.base_color_factor_r, raw.base_color_factor_g, raw.base_color_factor_b, raw.base_color_factor_a),

        // metallic roughness
        (raw.bitmask & BITMASK_METALIC_ROUGHNESS) != 0u,
        convert_texture_info(raw.metallic_roughness_tex_info),
        raw.metallic_factor,
        raw.roughness_factor,

        // normal
        (raw.bitmask & BITMASK_NORMAL) != 0u,
        convert_texture_info(raw.normal_tex_info),
        raw.normal_scale,

        // occlusion
        (raw.bitmask & BITMASK_OCCLUSION) != 0u,
        convert_texture_info(raw.occlusion_tex_info),
        raw.occlusion_strength,

        // emissive
        (raw.bitmask & BITMASK_EMISSIVE) != 0u,
        convert_texture_info(raw.emissive_tex_info),
        vec3<f32>(raw.emissive_factor_r, raw.emissive_factor_g, raw.emissive_factor_b),
        raw.emissive_strength,

        // color
        (raw.bitmask & BITMASK_COLOR) != 0u,
        raw.color_info,

        // specular
        (raw.bitmask & BITMASK_SPECULAR) != 0u,
        convert_texture_info(raw.specular_tex_info),
        raw.specular,

        // specular color
        (raw.bitmask & BITMASK_SPECULAR_COLOR) != 0u,
        convert_texture_info(raw.specular_color_tex_info),
        vec3<f32>(raw.specular_color_r, raw.specular_color_g, raw.specular_color_b),

        // ior
        raw.ior,

        // transmission
        (raw.bitmask & BITMASK_TRANSMISSION) != 0u,
        convert_texture_info(raw.transmission_tex_info),
        raw.transmission_factor,

        // volume thickness
        (raw.bitmask & BITMASK_VOLUME_THICKNESS) != 0u,
        convert_texture_info(raw.volume_thickness_tex_info),
        raw.volume_thickness_factor,
        raw.volume_attenuation_distance,
        vec3<f32>(raw.volume_attenuation_color_r, raw.volume_attenuation_color_g, raw.volume_attenuation_color_b),


    );
}
