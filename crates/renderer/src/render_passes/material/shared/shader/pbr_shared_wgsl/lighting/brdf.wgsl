// -------------------------------------------------------------
// PBR (metal/roughness) BRDF with IBL stubs (WGSL)
// Clean version: NO final saturate, safe for HDR + post tonemapping
// -------------------------------------------------------------

struct IblInfo {
    prefiltered_env_mip_count: u32,
    irradiance_mip_count: u32,
}

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
// Real IBL Sampling Functions
// -------------------------------------------------------------

// Sample irradiance map for diffuse IBL contribution
fn sampleIrradiance(
    n: vec3<f32>,
    irradiance_tex: texture_cube<f32>,
    irradiance_sampler: sampler
) -> vec3<f32> {
    // Use textureSampleLevel with mip 0 for compute shader compatibility
    return textureSampleLevel(irradiance_tex, irradiance_sampler, n, 0.0).rgb;
}

// Sample prefiltered environment map for specular IBL contribution
// Uses roughness to select appropriate mip level (split-sum approximation)
fn samplePrefilteredEnv(
    R: vec3<f32>,
    roughness: f32,
    filtered_env_tex: texture_cube<f32>,
    filtered_env_sampler: sampler,
    ibl_info: IblInfo
) -> vec3<f32> {
    // Map roughness to mip level (0 = sharpest reflection, max = most blurred)
    let max_mip = f32(ibl_info.prefiltered_env_mip_count - 1u);
    let mip_level = roughness * max_mip;
    return textureSampleLevel(filtered_env_tex, filtered_env_sampler, R, mip_level).rgb;
}

// Sample BRDF integration LUT for split-sum approximation
// Returns vec2(scale, bias) for F0 * scale + bias
fn sampleBRDFLUT(
    n_dot_v: f32,
    roughness: f32,
    brdf_lut_tex: texture_2d<f32>,
    brdf_lut_sampler: sampler
) -> vec2<f32> {
    let uv = vec2<f32>(saturate(n_dot_v), saturate(roughness));
    // Use textureSampleLevel with mip 0 for compute shader compatibility
    return textureSampleLevel(brdf_lut_tex, brdf_lut_sampler, uv, 0.0).rg;
}

// -------------------------------------------------------------
// Direct Lighting BRDF
// -------------------------------------------------------------
// Computes direct lighting contribution from a single light source
fn brdf_direct(color: PbrMaterialColor, light_brdf: LightBrdf, surface_to_camera: vec3<f32>) -> vec3<f32> {
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

    // Cook-Torrance BRDF
    let F = fresnel_schlick(v_dot_h, F0);
    let D = distribution_ggx(n_dot_h, alpha);
    let G = geometry_smith(n, v, l, alpha);

    let spec     = (D * G) / max(4.0 * n_dot_l * n_dot_v, EPSILON);
    let spec_col = F * spec;

    let k_d      = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let diff_col = k_d * base_color * (1.0 / PI);

    // Direct lighting output (Lo)
    return (diff_col + spec_col) * light_brdf.radiance * n_dot_l;
}

// -------------------------------------------------------------
// Image-Based Lighting (IBL)
// -------------------------------------------------------------
// Computes indirect lighting contribution from environment maps
fn brdf_ibl(
    color: PbrMaterialColor,
    normal: vec3<f32>,
    surface_to_camera: vec3<f32>,
    ibl_filtered_env_tex: texture_cube<f32>,
    ibl_filtered_env_sampler: sampler,
    ibl_irradiance_tex: texture_cube<f32>,
    ibl_irradiance_sampler: sampler,
    brdf_lut_tex: texture_2d<f32>,
    brdf_lut_sampler: sampler,
    ibl_info: IblInfo
) -> vec3<f32> {
    let n = safe_normalize(normal);
    let v = safe_normalize(surface_to_camera);

    let base_color = color.base.rgb;
    let metallic   = clamp(color.metallic_roughness.x, 0.0, 1.0);
    let rough_in   = clamp(color.metallic_roughness.y, 0.0, 1.0);
    let roughness  = max(rough_in, 0.04);

    let n_dot_v = max(dot(n, v), 1e-4);
    let F0 = mix(vec3<f32>(0.04), base_color, metallic);

    // Diffuse IBL (irradiance)
    let irradiance = sampleIrradiance(n, ibl_irradiance_tex, ibl_irradiance_sampler);
    let F_view     = fresnel_schlick(n_dot_v, F0);
    let k_d_indir  = (vec3<f32>(1.0) - F_view) * (1.0 - metallic);
    let Fd_indir   = k_d_indir * base_color * (1.0 / PI) * irradiance * color.occlusion;

    // Specular IBL (prefiltered environment + BRDF LUT)
    let R          = reflect(-v, n);
    let prefiltered = samplePrefilteredEnv(R, roughness, ibl_filtered_env_tex, ibl_filtered_env_sampler, ibl_info);
    let brdf_lut    = sampleBRDFLUT(n_dot_v, roughness, brdf_lut_tex, brdf_lut_sampler);
    let Fs_indir    = prefiltered * (F0 * brdf_lut.x + brdf_lut.y);

    // Return indirect lighting + emissive
    return Fd_indir + Fs_indir + color.emissive;
}
