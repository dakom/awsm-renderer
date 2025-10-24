// -------------------------------------------------------------
// PBR (metal/roughness) BRDF with Image-Based Lighting (WGSL)
// Implements Cook-Torrance specular BRDF with split-sum IBL approximation
// Safe for HDR workflows (no final saturate - tone mapping applied elsewhere)
// -------------------------------------------------------------

// -------------------------------------------------------------
// Microfacet BRDF Components
// -------------------------------------------------------------

// Fresnel-Schlick approximation: view-dependent reflectance
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    let ct = saturate(cos_theta);
    let one_minus = 1.0 - ct;
    return F0 + (1.0 - F0) * pow(one_minus, 5.0);
}

// GGX/Trowbridge-Reitz normal distribution function
fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let a  = max(alpha, 0.001);
    let a2 = a * a;
    let ndh = saturate(n_dot_h);
    let d  = ndh * ndh * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d + EPSILON);
}

// Schlick-GGX geometry function (single direction)
fn geometry_schlick_ggx(n_dot_x: f32, alpha: f32) -> f32 {
    let a = max(alpha, 0.001);
    let k = ((a + 1.0) * (a + 1.0)) * 0.125; // Direct lighting: k = (α+1)²/8
    let ndx = saturate(n_dot_x);
    return ndx / (ndx * (1.0 - k) + k);
}

// Smith geometry function (combines view and light directions)
fn geometry_smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, alpha: f32) -> f32 {
    let n_dot_v = saturate(dot(n, v));
    let n_dot_l = saturate(dot(n, l));
    return geometry_schlick_ggx(n_dot_v, alpha) * geometry_schlick_ggx(n_dot_l, alpha);
}

// -------------------------------------------------------------
// IBL Sampling Functions
// -------------------------------------------------------------

// Sample pre-convolved irradiance map for diffuse IBL
fn sampleIrradiance(
    n: vec3<f32>,
    irradiance_tex: texture_cube<f32>,
    irradiance_sampler: sampler
) -> vec3<f32> {
    return textureSampleLevel(irradiance_tex, irradiance_sampler, n, 0.0).rgb;
}

// Sample prefiltered environment map for specular IBL (split-sum approximation)
// Roughness selects mip level: 0 = sharp reflections, max = fully diffuse
fn samplePrefilteredEnv(
    R: vec3<f32>,
    roughness: f32,
    filtered_env_tex: texture_cube<f32>,
    filtered_env_sampler: sampler,
    ibl_info: IblInfo
) -> vec3<f32> {
    let max_mip = f32(ibl_info.prefiltered_env_mip_count - 1u);
    let mip_level = roughness * max_mip;
    return textureSampleLevel(filtered_env_tex, filtered_env_sampler, R, mip_level).rgb;
}

// Sample BRDF integration LUT (2D texture indexed by N·V and roughness)
// Returns (scale, bias) for computing F0 * scale + bias
fn sampleBRDFLUT(
    n_dot_v: f32,
    roughness: f32,
    brdf_lut_tex: texture_2d<f32>,
    brdf_lut_sampler: sampler
) -> vec2<f32> {
    let uv = vec2<f32>(saturate(n_dot_v), saturate(roughness));
    return textureSampleLevel(brdf_lut_tex, brdf_lut_sampler, uv, 0.0).rg;
}

// -------------------------------------------------------------
// Direct Lighting BRDF (Cook-Torrance)
// -------------------------------------------------------------
fn brdf_direct(color: PbrMaterialColor, light_brdf: LightBrdf, surface_to_camera: vec3<f32>) -> vec3<f32> {
    let n = safe_normalize(light_brdf.normal);
    let v = safe_normalize(surface_to_camera);
    let l = safe_normalize(light_brdf.light_dir);
    let h = safe_normalize(v + l);

    // Material properties
    let base_color = color.base.rgb;
    let metallic   = clamp(color.metallic_roughness.x, 0.0, 1.0);
    let roughness  = max(clamp(color.metallic_roughness.y, 0.0, 1.0), 0.04);
    let alpha      = roughness * roughness;

    // Lighting geometry
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 1e-4);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);

    // F0: base reflectivity at normal incidence (0.04 for dielectrics, base_color for metals)
    let F0 = mix(vec3<f32>(0.04), base_color, metallic);

    // Cook-Torrance specular BRDF: DFG / (4 * N·L * N·V)
    let F = fresnel_schlick(v_dot_h, F0);
    let D = distribution_ggx(n_dot_h, alpha);
    let G = geometry_smith(n, v, l, alpha);
    let specular = F * (D * G) / max(4.0 * n_dot_l * n_dot_v, EPSILON);

    // Lambertian diffuse (energy-conserving: scaled by (1-F) and non-metallic portion)
    let k_d = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let diffuse = k_d * base_color * (1.0 / PI);

    // Final radiance: (diffuse + specular) * incoming light * N·L
    return (diffuse + specular) * light_brdf.radiance * n_dot_l;
}

// -------------------------------------------------------------
// Image-Based Lighting (IBL) - Split-sum Approximation
// -------------------------------------------------------------
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

    // Material properties
    let base_color = color.base.rgb;
    let metallic   = clamp(color.metallic_roughness.x, 0.0, 1.0);
    let roughness  = max(clamp(color.metallic_roughness.y, 0.0, 1.0), 0.04);

    let n_dot_v = saturate(dot(n, v));
    let F0 = mix(vec3<f32>(0.04), base_color, metallic);

    // Diffuse IBL: irradiance * Lambertian BRDF * (1 - Fresnel) * (1 - metallic)
    let irradiance = sampleIrradiance(n, ibl_irradiance_tex, ibl_irradiance_sampler);
    let F_view = fresnel_schlick(n_dot_v, F0);
    let k_d = (vec3<f32>(1.0) - F_view) * (1.0 - metallic);
    let diffuse = k_d * base_color * (1.0 / PI) * irradiance * color.occlusion;

    // Specular IBL: prefiltered environment * (F0 * scale + bias) from BRDF LUT
    let R = reflect(-v, n);
    let prefiltered = samplePrefilteredEnv(R, roughness, ibl_filtered_env_tex, ibl_filtered_env_sampler, ibl_info);
    let brdf_lut = sampleBRDFLUT(n_dot_v, roughness, brdf_lut_tex, brdf_lut_sampler);
    let specular = prefiltered * (F0 * brdf_lut.x + brdf_lut.y);

    return diffuse + specular + color.emissive;
}
