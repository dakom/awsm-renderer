//--------------------------------------------------------------------
//  helpers (snake_case all the way)
//--------------------------------------------------------------------

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let d  = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (pi * d * d);
}

fn geometry_schlick_ggx(n_dot_x: f32, alpha: f32) -> f32 {
    let k = pow(alpha + 1.0, 2.0) * 0.125;   // (alpha+1)^2 / 8
    return n_dot_x / (n_dot_x * (1.0 - k) + k);
}

fn geometry_smith(n: vec3<f32>, v: vec3<f32>,
                  l: vec3<f32>, alpha: f32) -> f32 {
    let n_dot_v = saturate(dot(n, v));
    let n_dot_l = saturate(dot(n, l));
    return geometry_schlick_ggx(n_dot_v, alpha) *
           geometry_schlick_ggx(n_dot_l, alpha);
}

//--------------------------------------------------------------------
//  texture‑or‑factor fetch helpers
//--------------------------------------------------------------------
fn sample_base_color(base_color_factor: vec4<f32>) -> vec4<f32> {
    {% if material.has_base_color_tex %}
        let tex = textureSample(base_color_tex, base_color_sampler, input.base_color_uv);
        return tex * base_color_factor;
    {% else %}
        return base_color_factor;
    {% endif %}
}

fn sample_metal_rough(metallic_factor: f32, roughness_factor: f32) -> vec2<f32> { // x=metallic y=roughness

    {% if material.has_metallic_roughness_tex %}
        let tex = textureSample(metallic_roughness_tex, metallic_roughness_sampler, input.metallic_roughness_uv);
        return vec2<f32>(tex.b, tex.g) *
               vec2<f32>(1.0, 1.0) +          // texture is already linear
               vec2<f32>(0.0, 0.0);           // no factor in glTF spec
    {% else %}
        return vec2<f32>(metallic_factor,
                         roughness_factor);
    {% endif %}

}

fn sample_normal(n: vec3<f32>, normal_scale: f32) -> vec3<f32> {
    {% if material.has_normal_tex %}
        let tex = textureSample(normal_tex, normal_sampler, input.normal_uv);
        let raw = tex.xyz * 2.0 - 1.0;
        // Tangent‑space normal; assume matrix TBN in caller
        return normalize(raw * vec3<f32>(normal_scale, normal_scale, 1.0));
    {% else %}
        return n;
    {% endif %}
}

fn sample_occlusion(occlusion_strength: f32) -> f32 {
    {% if material.has_occlusion_tex %}
        let tex = textureSample(occlusion_tex, occlusion_sampler, input.occlusion_uv);
        return mix(1.0, tex.r, occlusion_strength);
    {% else %}
        return 1.0;
    {% endif %}
}

fn sample_emissive(emissive_factor: vec3<f32>) -> vec3<f32> {
    {% if material.has_emissive_tex %}
        let tex = textureSample(emissive_tex, emissive_sampler, input.emissive_uv);
        return tex.rgb * emissive_factor;
    {% else %}
        return emissive_factor;
    {% endif %}
}

struct LightBrdf {
    normal: vec3<f32>,
    n_dot_l: f32, 
    light_dir: vec3<f32>, 
    radiance: vec3<f32>, 
};

//--------------------------------------------------------------------
//  main brdf (same math as before, but snake_case)
//--------------------------------------------------------------------
fn brdf(material: PbrMaterial, light_brdf: LightBrdf, ambient: vec3<f32>, surface_to_camera: vec3<f32>) -> vec3<f32> {
    let n = light_brdf.normal;
    let v = surface_to_camera;
    let l = light_brdf.light_dir;
    let radiance = light_brdf.radiance;

    let base_color   = sample_base_color(material.base_color_factor);
    let mr           = sample_metal_rough(material.metallic_factor, material.roughness_factor);
    let metallic     = mr.x;
    let roughness    = saturate(mr.y);
    let alpha        = roughness * roughness;

    let h            = normalize(v + l);
    let n_dot_l      = saturate(dot(n, l));
    let n_dot_v      = saturate(dot(n, v));
    let n_dot_h      = saturate(dot(n, h));
    let v_dot_h      = saturate(dot(v, h));

    // fresnel base reflectance
    let f0 = mix(vec3<f32>(0.04), base_color.rgb, metallic);

    // specular
    let f     = fresnel_schlick(v_dot_h, f0);
    let d     = distribution_ggx(n_dot_h, alpha);
    let g     = geometry_smith(n, v, l, alpha);
    let spec  = (d * g) / max(4.0 * n_dot_l * n_dot_v, 0.001);
    let spec_col = f * spec;

    // diffuse
    let k_s   = f;
    let k_d   = (1.0 - k_s) * (1.0 - metallic);
    let diff_col = k_d * base_color.rgb * (1.0 / pi);

    // light contribution
    let lo = (diff_col + spec_col) * radiance * n_dot_l;

    // ambient + occlusion
    let ao = sample_occlusion(material.occlusion_strength);
    let ambient_col = (diff_col + spec_col) * ambient * ao;

    // emissive
    let emissive = sample_emissive(material.emissive_factor);

    return lo + ambient_col + emissive;
}
