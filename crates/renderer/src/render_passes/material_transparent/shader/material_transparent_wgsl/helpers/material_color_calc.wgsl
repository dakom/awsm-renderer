// Fragment shader versions of PBR material color sampling
// These functions work with interpolated vertex data (no barycentrics/attribute buffers needed)
// Hardware automatically handles mip level selection via screen-space derivatives

// Main function: Sample all PBR material textures and return combined material properties
// Returns PbrMaterialColor with perturbed normal (use material_color.normal for lighting!)
fn pbr_get_material_color(
    material: PbrMaterial,
    world_normal: vec3<f32>,
    world_tangent: vec4<f32>,
    fragment_input: FragmentInput
) -> PbrMaterialColor {
    // Load extension data on-demand from indices
    let emissive_strength = pbr_material_load_emissive_strength(material.emissive_strength_index);
    let ior = pbr_material_load_ior(material.ior_index);
    let specular = pbr_material_load_specular(material.specular_index);
    let transmission = pbr_material_load_transmission(material.transmission_index);
    let volume = pbr_material_load_volume(material.volume_index);

    var base = pbr_material_base_color(material, fragment_input);

    // Multiply base color by vertex color if material has color info
    {%- if color_sets.is_some() %}
        let vertex_color_info = pbr_material_load_vertex_color_info(material.vertex_color_info_index);
        if vertex_color_info.set_index != 0u {
            base *= vertex_color(vertex_color_info, fragment_input);
        }
    {% endif %}

    if material.alpha_mode == ALPHA_MODE_MASK {
        // Discard fragment if alpha below cutoff
        if base.a < material.alpha_cutoff {
            discard;
        } else {
            base.a = 1.0;
        }
    }

    let metallic_roughness = pbr_material_metallic_roughness(material, fragment_input);
    let normal = pbr_normal(material, world_normal, world_tangent, fragment_input);
    let occlusion = pbr_occlusion(material, fragment_input);
    let emissive = pbr_emissive(material, emissive_strength, fragment_input);
    let specular_factor = pbr_specular(specular, fragment_input);
    let specular_color_factor = pbr_specular_color(specular, fragment_input);
    let transmission_factor = pbr_transmission(transmission, fragment_input);
    let volume_thickness = pbr_volume_thickness(volume, fragment_input);

    return PbrMaterialColor(
        base,
        metallic_roughness,
        normal,
        occlusion,
        emissive,
        specular_factor,
        specular_color_factor,
        ior,
        transmission_factor,
        volume_thickness,
        volume.attenuation_distance,
        volume.attenuation_color
    );
}

// Sample base color texture and apply material factor
fn pbr_material_base_color(
    material: PbrMaterial,
    fragment_input: FragmentInput
) -> vec4<f32> {
    var color = material.base_color_factor;
    if material.base_color_tex_info.exists {
        let uv = texture_uv(material.base_color_tex_info, fragment_input);
        color *= texture_pool_sample(material.base_color_tex_info, uv);
    }
    return color;
}

// Sample metallic-roughness texture and apply material factors
// glTF uses B channel for metallic, G channel for roughness
fn pbr_material_metallic_roughness(
    material: PbrMaterial,
    fragment_input: FragmentInput
) -> vec2<f32> {
    var color = vec2<f32>(material.metallic_factor, material.roughness_factor);
    if material.metallic_roughness_tex_info.exists {
        let uv = texture_uv(material.metallic_roughness_tex_info, fragment_input);
        let tex = texture_pool_sample(material.metallic_roughness_tex_info, uv);
        color *= vec2<f32>(tex.b, tex.g);
    }
    return color;
}

// Apply normal mapping using interpolated tangent space basis from vertex shader
// Much simpler than compute version - relies on vertex shader providing correct tangents
fn pbr_normal(
    material: PbrMaterial,
    world_normal: vec3<f32>,
    world_tangent: vec4<f32>,  // w = handedness (+1 or -1)
    fragment_input: FragmentInput
) -> vec3<f32> {
    if !material.normal_tex_info.exists {
        return normalize(world_normal);
    }

    // Sample normal map and unpack from [0,1] to [-1,1] range
    let uv = texture_uv(material.normal_tex_info, fragment_input);
    let tex = texture_pool_sample(material.normal_tex_info, uv);
    let tangent_normal = vec3<f32>(
        (tex.r * 2.0 - 1.0) * material.normal_scale,
        (tex.g * 2.0 - 1.0) * material.normal_scale,
        tex.b * 2.0 - 1.0,
    );

    // Build TBN matrix from interpolated vertex data
    let N = normalize(world_normal);
    let T = normalize(world_tangent.xyz);
    let B = cross(N, T) * world_tangent.w;
    let tbn = mat3x3<f32>(T, B, N);

    // Transform tangent-space normal to world space
    return normalize(tbn * tangent_normal);
}

// Sample occlusion texture and apply strength factor
fn pbr_occlusion(
    material: PbrMaterial,
    fragment_input: FragmentInput
) -> f32 {
    var occlusion = 1.0;
    if material.occlusion_tex_info.exists {
        let uv = texture_uv(material.occlusion_tex_info, fragment_input);
        let tex = texture_pool_sample(material.occlusion_tex_info, uv);
        occlusion = mix(1.0, tex.r, material.occlusion_strength);
    }
    return occlusion;
}

// Sample emissive texture and apply factors
fn pbr_emissive(
    material: PbrMaterial,
    emissive_strength: f32,
    fragment_input: FragmentInput
) -> vec3<f32> {
    var color = material.emissive_factor;
    if material.emissive_tex_info.exists {
        let uv = texture_uv(material.emissive_tex_info, fragment_input);
        color *= texture_pool_sample(material.emissive_tex_info, uv).rgb;
    }
    color *= emissive_strength;
    return color;
}

// Sample specular texture (alpha channel) and apply factor
fn pbr_specular(
    specular: PbrSpecular,
    fragment_input: FragmentInput
) -> f32 {
    var factor = specular.factor;
    if specular.tex_info.exists {
        let uv = texture_uv(specular.tex_info, fragment_input);
        factor *= texture_pool_sample(specular.tex_info, uv).a;
    }
    return factor;
}

// Sample specular color texture (RGB) and apply factor
fn pbr_specular_color(
    specular: PbrSpecular,
    fragment_input: FragmentInput
) -> vec3<f32> {
    var color = specular.color_factor;
    if specular.color_tex_info.exists {
        let uv = texture_uv(specular.color_tex_info, fragment_input);
        color *= texture_pool_sample(specular.color_tex_info, uv).rgb;
    }
    return color;
}

// Sample transmission texture (R channel) and apply factor
fn pbr_transmission(
    transmission: PbrTransmission,
    fragment_input: FragmentInput
) -> f32 {
    // Early exit: if no texture and factor is 0, skip entirely
    if (!transmission.tex_info.exists && transmission.factor == 0.0) {
        return 0.0;
    }
    var factor = transmission.factor;
    if transmission.tex_info.exists {
        let uv = texture_uv(transmission.tex_info, fragment_input);
        factor *= texture_pool_sample(transmission.tex_info, uv).r;
    }
    return factor;
}

// Sample volume thickness texture (G channel) and apply factor
fn pbr_volume_thickness(
    volume: PbrVolume,
    fragment_input: FragmentInput
) -> f32 {
    // Early exit: no volume if thickness is 0 and no texture
    if (!volume.thickness_tex_info.exists && volume.thickness_factor == 0.0) {
        return 0.0;
    }
    var thickness = volume.thickness_factor;
    if volume.thickness_tex_info.exists {
        let uv = texture_uv(volume.thickness_tex_info, fragment_input);
        // Volume thickness is stored in the G channel per glTF spec
        thickness *= texture_pool_sample(volume.thickness_tex_info, uv).g;
    }
    return thickness;
}

// ============================================================================
// Unlit Material Color Computation
// ============================================================================

// Compute unlit material color for fragment shader
fn unlit_get_material_color(
    material: UnlitMaterial,
    fragment_input: FragmentInput
) -> UnlitMaterialColor {
    // Compute base color
    var base = material.base_color_factor;
    if material.base_color_tex_info.exists {
        let uv = texture_uv(material.base_color_tex_info, fragment_input);
        base *= texture_pool_sample(material.base_color_tex_info, uv);
    }

    // Handle alpha modes
    if material.alpha_mode == ALPHA_MODE_MASK {
        if base.a < material.alpha_cutoff {
            discard;
        } else {
            base.a = 1.0;
        }
    }

    // Compute emissive
    var emissive = material.emissive_factor;
    if material.emissive_tex_info.exists {
        let uv = texture_uv(material.emissive_tex_info, fragment_input);
        emissive *= texture_pool_sample(material.emissive_tex_info, uv).rgb;
    }

    return UnlitMaterialColor(base, emissive);
}
