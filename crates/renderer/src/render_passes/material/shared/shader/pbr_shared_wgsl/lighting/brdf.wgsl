// -------------------------------------------------------------
// PBR (metal/roughness) BRDF with IBL stubs (WGSL)
// Clean version: NO final saturate, safe for HDR + post tonemapping
// -------------------------------------------------------------


// --- microfacet helpers ---
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    let ct = saturate(cos_theta);
    let one_minus = 1.0 - ct;
    return F0 + (1.0 - F0) * pow(one_minus, 5.0);
}

fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let a  = max(alpha, 0.001);
    let a2 = a * a;
    let ndh = saturate(n_dot_h);
    let d  = ndh * ndh * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d + EPSILON);
}

fn geometry_schlick_ggx(n_dot_x: f32, alpha: f32) -> f32 {
    let a = max(alpha, 0.001);
    let k = ((a + 1.0) * (a + 1.0)) * 0.125; // (alpha+1)^2 / 8
    let ndx = saturate(n_dot_x);
    return ndx / (ndx * (1.0 - k) + k);
}

fn geometry_smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, alpha: f32) -> f32 {
    let n_dot_v = saturate(dot(n, v));
    let n_dot_l = saturate(dot(n, l));
    return geometry_schlick_ggx(n_dot_v, alpha) * geometry_schlick_ggx(n_dot_l, alpha);
}

// -------------------------------------------------------------
// IBL STUBS (replace with real env sampling later)
// -------------------------------------------------------------
const ENV_DIFFUSE_COLOR  : vec3<f32> = vec3<f32>(0.22, 0.24, 0.26);
const ENV_SPECULAR_COLOR : vec3<f32> = vec3<f32>(1.0, 0.98, 0.95);
const ENV_INTENSITY_DIFF : f32 = 0.3;
const ENV_INTENSITY_SPEC : f32 = 0.7;

fn sampleIrradianceStub(n: vec3<f32>) -> vec3<f32> {
    let hemi = 0.5 * (dot(n, vec3<f32>(0.0, 1.0, 0.0)) + 1.0);
    let tint = mix(vec3<f32>(0.7, 0.7, 0.75), vec3<f32>(1.0, 1.0, 1.0), hemi);
    return ENV_DIFFUSE_COLOR * tint * ENV_INTENSITY_DIFF;
}

fn samplePrefilteredEnvStub(R: vec3<f32>, roughness: f32) -> vec3<f32> {
    let gloss = 1.0 - saturate(roughness);
    let facing = saturate(R.y * 0.5 + 0.5);
    let gain = mix(0.35, 1.0, gloss * facing);
    return ENV_SPECULAR_COLOR * gain * ENV_INTENSITY_SPEC;
}

fn sampleBRDFLUTStub(n_dot_v: f32, roughness: f32) -> vec2<f32> {
    let ndv = saturate(n_dot_v);
    let r   = clamp(roughness, 0.0, 1.0);
    let x = mix(0.95, 0.45, r) * mix(1.0, 0.7, (1.0 - ndv));
    let y = mix(0.04, 0.55, r) * mix(0.6, 1.0, (1.0 - ndv));
    return vec2<f32>(x, y);
}

// -------------------------------------------------------------
// Main BRDF
// -------------------------------------------------------------
//
fn brdf(color: PbrMaterialColor, light_brdf: LightBrdf, surface_to_camera: vec3<f32>) -> vec3<f32> {
    let n = safe_normalize(light_brdf.normal);
    let v = safe_normalize(surface_to_camera);
    let l = safe_normalize(light_brdf.light_dir);
    let h = safe_normalize(v + l);

    let base_color = color.base.rgb;    // linear
    let metallic   = clamp(color.metallic_roughness.x, 0.0, 1.0);
    let rough_in   = clamp(color.metallic_roughness.y, 0.0, 1.0);
    let roughness  = max(rough_in, 0.04);
    let alpha      = max(roughness * roughness, 1e-4);

    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 1e-4);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);

    let F0 = mix(vec3<f32>(0.04), base_color, metallic);

    // Direct
    let F = fresnel_schlick(v_dot_h, F0);
    let D = distribution_ggx(n_dot_h, alpha);
    let G = geometry_smith(n, v, l, alpha);

    let spec     = (D * G) / max(4.0 * n_dot_l * n_dot_v, EPSILON);
    let spec_col = F * spec;

    let k_d      = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let diff_col = k_d * base_color * (1.0 / PI);

    let Lo = (diff_col + spec_col) * light_brdf.radiance * n_dot_l;

    // IBL stubs (unchanged, but use safe normal and clamped ndv)
    let irradiance = sampleIrradianceStub(n);
    let F_view     = fresnel_schlick(n_dot_v, F0);
    let k_d_indir  = (vec3<f32>(1.0) - F_view) * (1.0 - metallic);
    let Fd_indir   = k_d_indir * base_color * (1.0 / PI) * irradiance * color.occlusion;

    let R          = reflect(-v, n);
    let prefiltered = samplePrefilteredEnvStub(R, roughness);
    let brdf_lut    = sampleBRDFLUTStub(n_dot_v, roughness);
    let Fs_indir    = prefiltered * (F0 * brdf_lut.x + brdf_lut.y);

    return Lo + Fd_indir + Fs_indir + color.emissive;
}
