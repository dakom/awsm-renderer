struct PbrMaterialRaw {
    base_color_factor: vec4<f32>,  // 16 B

    metallic_factor     : f32,
    roughness_factor    : f32,
    normal_scale        : f32,
    occlusion_strength  : f32,        // together 16 B

    emissive_factor     : vec3<f32>,
    _pad0               : f32,        // 16 B

    alpha_mode          : u32,
    alpha_cutoff        : f32,
    double_sided        : u32,
    _pad1               : u32,        // 16 B
};

@group(1) @binding(1)
var<uniform> u_material: PbrMaterialRaw;

struct PbrMaterial {
    base_color_factor : vec4<f32>,
    metallic_factor   : f32,
    roughness_factor  : f32,
    normal_scale : f32,
    occlusion_strength : f32,
    emissive_factor   : vec3<f32>,
    alpha_mode : u32,
    alpha_cutoff : f32,
    double_sided : bool,
};

fn toMaterial(raw : PbrMaterialRaw) -> PbrMaterial {
    return PbrMaterial(
        raw.base_color_factor,
        raw.metallic_factor,
        raw.roughness_factor,
        raw.normal_scale,
        raw.occlusion_strength,
        raw.emissive_factor,
        raw.alpha_mode,
        raw.alpha_cutoff,
        raw.double_sided != 0u
    );
}