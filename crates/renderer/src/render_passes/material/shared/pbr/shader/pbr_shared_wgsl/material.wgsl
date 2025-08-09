struct MaterialRaw {
    // Basic properties, 15 * 4 = 60 bytes
    offset: u32,
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
    // 256 - (60 + 120 + 4) = 72 bytes padding, as 8 u32s
    padding: array<u32, 18>
};

struct Material {
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

fn convert_material(raw: MaterialRaw) -> Material {
    const TEXTURE_BITMASK_BASE_COLOR: u32 = 1u;
    const TEXTURE_BITMASK_METALIC_ROUGHNESS: u32 = 2u;
    const TEXTURE_BITMASK_NORMAL: u32 = 4u;
    const TEXTURE_BITMASK_OCCLUSION: u32 = 8u;
    const TEXTURE_BITMASK_EMISSIVE: u32 = 16u;

    return Material(
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

// 24 bytes
struct TextureInfoRaw {
    pixel_offset_x: u32,
    pixel_offset_y: u32,
    width: u32,
    height: u32,
    atlas_layer_index: u32,
    entry_empty_index: u32,
}

struct TextureInfo {
    pixel_offset: vec2<u32>,
    size: vec2<u32>,
    atlas_index: u32,
    layer_index: u32,
    entry_index: u32
}

fn convert_texture_info(raw: TextureInfoRaw) -> TextureInfo {
    return TextureInfo(
        vec2<u32>(raw.pixel_offset_x, raw.pixel_offset_y),
        vec2<u32>(raw.width, raw.height),
        raw.atlas_layer_index & 0xFFFFu,           // atlas_index (16 bits)
        (raw.atlas_layer_index >> 16u) & 0xFFFFu, // layer_index (16 bits)
        raw.entry_empty_index & 0xFFFFu           // entry_index (16 bits)
    );
}