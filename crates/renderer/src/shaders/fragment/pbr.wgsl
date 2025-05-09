{% for binding in fragment_buffer_bindings %}
    @group({{ binding.group }}) @binding({{ binding.index }}) var {{ binding.name }}: {{ binding.data_type }};
{% endfor %}

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

struct PbrMaterial {
    base_color : vec4<f32>,
    metallic   : f32,
    roughness  : f32,
    normal_scale : f32,
    occlusion : f32,
    emissive   : vec3<f32>,
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


@group(1) @binding(1)
var<uniform> u_material: PbrMaterialRaw;


@fragment
fn frag_main(input: FragmentInput) -> @location(0) vec4<f32> {
    var material = toMaterial(u_material);

    {% if material.has_base_color %}
        var color = textureSample(base_color_tex, base_color_sampler, input.base_color_uv);
    {% else %}
        var color = material.base_color;
    {% endif %}

    return color;
}
