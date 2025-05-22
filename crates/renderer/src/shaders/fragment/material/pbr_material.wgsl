@group(1) @binding(1)
var<uniform> u_material: PbrMaterialRaw;

struct PbrMaterialRaw {
    base_color_factor: vec4<f32>,  // 16 B

    metallic_factor     : f32,
    roughness_factor    : f32,
    normal_scale        : f32,
    occlusion_strength  : f32,        // together 16 B

    emissive_factor     : vec3<f32>,
    _pad0               : f32,        // 16 B

    // 0: Opaque
    // 1: Mask (alpha cutoff is set separately)
    // 2: Blend
    alpha_mode          : u32,
    alpha_cutoff        : f32,
    double_sided        : u32,
    _pad1               : u32,        // 16 B
};

struct PbrMaterial {
    base_color: vec4<f32>,
    metallic_roughness: vec2<f32>,
    normal: vec3<f32>,
    occlusion: f32,
    emissive: vec3<f32>,
    double_sided : bool,
};

fn getMaterial(input: FragmentInput) -> PbrMaterial {
    {% if material.has_base_color_tex %}
        let base_color_uv = input.base_color_uv;
    {% else %}
        let base_color_uv = vec2<f32>(0.0, 0.0);
    {% endif %}

    {% if material.has_metallic_roughness_tex %}
        let metallic_roughness_uv = input.metallic_roughness_uv;
    {% else %}
        let metallic_roughness_uv = vec2<f32>(0.0, 0.0);
    {% endif %}

    {% if material.has_normal_tex %}
        let normal_uv = input.normal_uv;
    {% else %}
        let normal_uv = vec2<f32>(0.0, 0.0);
    {% endif %}

    {% if material.has_occlusion_tex %}
        let occlusion_uv = input.occlusion_uv;
    {% else %}
        let occlusion_uv = vec2<f32>(0.0, 0.0);
    {% endif %}

    {% if material.has_emissive_tex %}
        let emissive_uv = input.emissive_uv;
    {% else %}
        let emissive_uv = vec2<f32>(0.0, 0.0);
    {% endif %}


    let base_color = sample_base_color(u_material.base_color_factor, base_color_uv);
    {% if material.has_alpha_mask %}
        // early discard as soon as possible, to avoid expensive calculations
        if base_color.a < u_material.alpha_cutoff {
            discard;
        }
    {% endif %}
    let metallic_roughness = sample_metal_rough(u_material.metallic_factor, u_material.roughness_factor, metallic_roughness_uv);
    let normal = sample_normal(input.world_normal, u_material.normal_scale, normal_uv);
    let occlusion = sample_occlusion(u_material.occlusion_strength, occlusion_uv);
    let emissive = sample_emissive(u_material.emissive_factor, emissive_uv);


    return PbrMaterial(
        base_color,
        metallic_roughness,
        normal,
        occlusion,
        emissive,
        u_material.double_sided != 0u
    );
}


//--------------------------------------------------------------------
//  texture‑or‑factor fetch helpers
//--------------------------------------------------------------------
fn sample_base_color(base_color_factor: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    {% if material.has_base_color_tex %}
        let tex = textureSample(base_color_tex, base_color_sampler, uv);
        var color = tex * base_color_factor;
    {% else %}
        var color = base_color_factor;
    {% endif %}

    // alpha_mode: 0=opaque, 1=mask, 2=blend
    if u_material.alpha_mode == 0u {
        color.a = 1.0;
    }


    return color;
}

fn sample_metal_rough(metallic_factor: f32, roughness_factor: f32, uv: vec2<f32>) -> vec2<f32> { // x=metallic y=roughness

    {% if material.has_metallic_roughness_tex %}
        let tex = textureSample(metallic_roughness_tex, metallic_roughness_sampler, uv);
        return vec2<f32>(tex.b, tex.g) *
               vec2<f32>(1.0, 1.0) +          // texture is already linear
               vec2<f32>(0.0, 0.0);           // no factor in glTF spec
    {% else %}
        return vec2<f32>(metallic_factor,
                         roughness_factor);
    {% endif %}

}

fn sample_normal(n: vec3<f32>, normal_scale: f32, uv: vec2<f32>) -> vec3<f32> {
    {% if material.has_normal_tex %}
        let tex = textureSample(normal_tex, normal_sampler, uv);
        let raw = tex.xyz * 2.0 - 1.0;
        // Tangent‑space normal; assume matrix TBN in caller
        return normalize(raw * vec3<f32>(normal_scale, normal_scale, 1.0));
    {% else %}
        return n;
    {% endif %}
}

fn sample_occlusion(occlusion_strength: f32, uv: vec2<f32>) -> f32 {
    {% if material.has_occlusion_tex %}
        let tex = textureSample(occlusion_tex, occlusion_sampler, uv);
        return mix(1.0, tex.r, occlusion_strength);
    {% else %}
        return 1.0;
    {% endif %}
}

fn sample_emissive(emissive_factor: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    {% if material.has_emissive_tex %}
        let tex = textureSample(emissive_tex, emissive_sampler, uv);
        return tex.rgb * emissive_factor;
    {% else %}
        return emissive_factor;
    {% endif %}
}
