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
    var base = pbr_material_base_color(material, fragment_input);

    // Multiply base color by vertex color if material has color info
    {%- if color_sets.is_some() %}
        if material.has_color_info {
            base *= vertex_color(material.color_info, fragment_input);
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
    let emissive = pbr_emissive(material, fragment_input);
    let specular = pbr_specular(material, fragment_input);
    let specular_color = pbr_specular_color(material, fragment_input);

    return PbrMaterialColor(
        base,
        metallic_roughness,
        normal,
        occlusion,
        emissive,
        specular,
        specular_color
    );
}

// Sample base color texture and apply material factor
fn pbr_material_base_color(
    material: PbrMaterial,
    fragment_input: FragmentInput
) -> vec4<f32> {
    var color = material.base_color_factor;
    if material.has_base_color_texture {
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
    if material.has_metallic_roughness_texture {
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
    if !material.has_normal_texture {
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
    if material.has_occlusion_texture {
        let uv = texture_uv(material.occlusion_tex_info, fragment_input);
        let tex = texture_pool_sample(material.occlusion_tex_info, uv);
        occlusion = mix(1.0, tex.r, material.occlusion_strength);
    }
    return occlusion;
}

// Sample emissive texture and apply factors
fn pbr_emissive(
    material: PbrMaterial,
    fragment_input: FragmentInput
) -> vec3<f32> {
    var color = material.emissive_factor;
    if material.has_emissive_texture {
        let uv = texture_uv(material.emissive_tex_info, fragment_input);
        color *= texture_pool_sample(material.emissive_tex_info, uv).rgb;
    }
    color *= material.emissive_strength;
    return color;
}

// Sample specular texture (alpha channel) and apply factor
fn pbr_specular(
    material: PbrMaterial,
    fragment_input: FragmentInput
) -> f32 {
    var specular = material.specular_factor;
    if material.has_specular_texture {
        let uv = texture_uv(material.specular_tex_info, fragment_input);
        specular *= texture_pool_sample(material.specular_tex_info, uv).a;
    }
    return specular;
}

// Sample specular color texture (RGB) and apply factor
fn pbr_specular_color(
    material: PbrMaterial,
    fragment_input: FragmentInput
) -> vec3<f32> {
    var color = material.specular_color_factor;
    if material.has_specular_color_texture {
        let uv = texture_uv(material.specular_color_tex_info, fragment_input);
        color *= texture_pool_sample(material.specular_color_tex_info, uv).rgb;
    }
    return color;
}
