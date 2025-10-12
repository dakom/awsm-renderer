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


struct LightBrdf {
    normal: vec3<f32>,
    n_dot_l: f32,
    light_dir: vec3<f32>,
    radiance: vec3<f32>,
};


//--------------------------------------------------------------------
//  main brdf (same math as before, but snake_case)
//--------------------------------------------------------------------
fn brdf(color: PbrMaterialColor, light_brdf: LightBrdf, ambient: vec3<f32>, surface_to_camera: vec3<f32>) -> vec3<f32> {
    let n = light_brdf.normal;
    let v = surface_to_camera;
    let l = light_brdf.light_dir;
    let radiance = light_brdf.radiance;

    let base_color_rgb   = color.base.rgb;
    let mr           = color.metallic_roughness;
    let metallic     = mr.x;
    let roughness    = saturate(mr.y);
    let alpha        = roughness * roughness;

    let h            = normalize(v + l);
    let n_dot_l      = saturate(dot(n, l));
    let n_dot_v      = saturate(dot(n, v));
    let n_dot_h      = saturate(dot(n, h));
    let v_dot_h      = saturate(dot(v, h));

    // fresnel base reflectance
    let f0 = mix(vec3<f32>(0.04), base_color_rgb, metallic);

    // specular
    let f     = fresnel_schlick(v_dot_h, f0);
    let d     = distribution_ggx(n_dot_h, alpha);
    let g     = geometry_smith(n, v, l, alpha);
    let spec  = (d * g) / max(4.0 * n_dot_l * n_dot_v, 0.001);
    let spec_col = f * spec;

    // diffuse
    let k_s   = f;
    let k_d   = (1.0 - k_s) * (1.0 - metallic);
    let diff_col = k_d * base_color_rgb * (1.0 / pi);

    // light contribution
    let lo = (diff_col + spec_col) * radiance * n_dot_l;

    // ambient + occlusion
    let ao = color.occlusion;
    let ambient_col = (diff_col + spec_col) * ambient * ao;

    // emissive
    let emissive = color.emissive;

    return lo + ambient_col + emissive;
}
