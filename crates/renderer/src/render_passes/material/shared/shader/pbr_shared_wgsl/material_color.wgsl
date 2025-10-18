// --- PBR Material Color STARTS HERE ---
struct PbrMaterialColor {
    base: vec4<f32>,
    metallic_roughness: vec2<f32>,
    normal: vec3<f32>,
    occlusion: f32,
    emissive: vec3<f32>,
};

fn pbr_get_material_color(
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    triangle_index: u32,
    material: PbrMaterial,
    barycentric: vec3<f32>,
    vertex_attribute_stride: u32,
    mip_levels: PbrMaterialMipLevels,
    world_normal: vec3<f32>,
) -> PbrMaterialColor {

    let base = _pbr_material_base_color(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.base_color_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.base_color,
    );

    let metallic_roughness = _pbr_material_metallic_roughness_color (
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.metallic_roughness_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.metallic_roughness,
    );

    let normal = _pbr_normal_color(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.normal_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.normal,
        world_normal
    );

    let occlusion = _pbr_occlusion_color(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.occlusion_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.occlusion,
    );

    let emissive = _pbr_material_emissive_color(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.emissive_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.emissive,
    );

    return PbrMaterialColor(
        base,
        metallic_roughness,
        normal,
        occlusion,
        emissive,
    );
}
// Base Color
fn _pbr_material_base_color(material: PbrMaterial, attribute_uv: vec2<f32>, mip_level: f32) -> vec4<f32> {
    var color = material.base_color_factor;
    if material.has_base_color_texture {
        color *=
            texture_load_atlas(material.base_color_tex_info, attribute_uv, mip_level);
    }

    // compute pass only deals with fully opaque
    // mask and blend are handled in the fragment shader
    color.a = 1.0;

    return color;
}

fn _pbr_material_metallic_roughness_color(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    mip_level: f32,
) -> vec2<f32> {
    var color = vec2<f32>(material.metallic_factor, material.roughness_factor);
    if material.has_metallic_roughness_texture {
        let tex = texture_load_atlas(material.metallic_roughness_tex_info, attribute_uv, mip_level);
        // glTF uses B channel for metallic, G channel for roughness
        color *= vec2<f32>(tex.b, tex.g);
    }
    return color;
}

fn _pbr_normal_color(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    mip_level: f32,
    world_normal: vec3<f32>,
) -> vec3<f32> {
    let normal_scale = material.normal_scale;
    if material.has_normal_texture {
        let tex = texture_load_atlas(material.normal_tex_info, attribute_uv, mip_level);
        // normal map is in tangent space, so we need to transform it to world space
        let tangent_normal = normalize(vec3<f32>(tex.r * 2.0 - 1.0, tex.g * 2.0 - 1.0, tex.b));
        // For simplicity, assume TBN matrix is identity, so we just return the tangent normal
        // TODO: construct the TBN matrix from vertex attributes
        return normalize(tangent_normal * vec3<f32>(normal_scale, normal_scale, 1.0));
    } else {
        return world_normal;
    }
}

fn _pbr_occlusion_color(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    mip_level: f32,
) -> f32 {
    var occlusion = 1.0;
    if material.has_occlusion_texture {
        let tex = texture_load_atlas(material.occlusion_tex_info, attribute_uv, mip_level);
        occlusion = mix(1.0, tex.r, material.occlusion_strength);
    }
    return occlusion;
}

fn _pbr_material_emissive_color(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    mip_level: f32,
) -> vec3<f32> {
    var color = material.emissive_factor;
    if material.has_emissive_texture {
        color *=
            texture_load_atlas(material.emissive_tex_info, attribute_uv, mip_level).rgb;
    }
    return color;
}

// --- Optional gradient-sampling variants (append) ----------------------------

// Versions of the samplers that use gradients (atlas-aware).
// Requires a MipCache built at the call-site (see compute snippet below).

fn pbr_get_material_color_with_grads(
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    material: PbrMaterial,
    barycentric: vec3<f32>,
    vertex_attribute_stride: u32,
    cache: MipCache,
    world_normal: vec3<f32>
) -> PbrMaterialColor {
    let uv_base = texture_uv(attribute_data_offset, triangle_indices, barycentric, material.base_color_tex_info, vertex_attribute_stride);
    let uv_mr   = texture_uv(attribute_data_offset, triangle_indices, barycentric, material.metallic_roughness_tex_info, vertex_attribute_stride);
    let uv_n    = texture_uv(attribute_data_offset, triangle_indices, barycentric, material.normal_tex_info, vertex_attribute_stride);
    let uv_occ  = texture_uv(attribute_data_offset, triangle_indices, barycentric, material.occlusion_tex_info, vertex_attribute_stride);
    let uv_e    = texture_uv(attribute_data_offset, triangle_indices, barycentric, material.emissive_tex_info, vertex_attribute_stride);

    let base = _pbr_material_base_color_grad(material, uv_base, triangle_indices, attribute_data_offset, vertex_attribute_stride, cache);
    let mr   = _pbr_material_metallic_roughness_color_grad(material, uv_mr, triangle_indices, attribute_data_offset, vertex_attribute_stride, cache);
    let nrm  = _pbr_normal_color_grad(material, uv_n, triangle_indices, attribute_data_offset, vertex_attribute_stride, cache, world_normal);
    let occ  = _pbr_occlusion_color_grad(material, uv_occ, triangle_indices, attribute_data_offset, vertex_attribute_stride, cache);
    let emis = _pbr_material_emissive_color_grad(material, uv_e, triangle_indices, attribute_data_offset, vertex_attribute_stride, cache);

    return PbrMaterialColor(base, mr, nrm, occ, emis);
}

fn _pbr_material_base_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    cache: MipCache
) -> vec4<f32> {
    var color = material.base_color_factor;
    if material.has_base_color_texture {
        let grads = get_atlas_gradients(material.base_color_tex_info,
                                        triangle_indices,
                                        attribute_data_offset,
                                        vertex_attribute_stride,
                                        cache);
        color *= texture_sample_grad_atlas(material.base_color_tex_info, attribute_uv, grads);
    }
    color.a = 1.0;
    return color;
}

fn _pbr_material_metallic_roughness_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    cache: MipCache
) -> vec2<f32> {
    var mr = vec2<f32>(material.metallic_factor, material.roughness_factor);
    if material.has_metallic_roughness_texture {
        let grads = get_atlas_gradients(material.metallic_roughness_tex_info,
                                        triangle_indices,
                                        attribute_data_offset,
                                        vertex_attribute_stride,
                                        cache);
        let tex = texture_sample_grad_atlas(material.metallic_roughness_tex_info, attribute_uv, grads);
        mr *= vec2<f32>(tex.b, tex.g); // glTF: metallic=B, roughness=G
    }
    return mr;
}

fn _pbr_normal_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    cache: MipCache,
    world_normal: vec3<f32>
) -> vec3<f32> {
    if material.has_normal_texture {
        let grads = get_atlas_gradients(material.normal_tex_info,
                                        triangle_indices,
                                        attribute_data_offset,
                                        vertex_attribute_stride,
                                        cache);
        let tex = texture_sample_grad_atlas(material.normal_tex_info, attribute_uv, grads);
        let tn  = normalize(vec3<f32>(tex.r * 2.0 - 1.0, tex.g * 2.0 - 1.0, tex.b));
        let s   = material.normal_scale;
        return normalize(tn * vec3<f32>(s, s, 1.0));
    }
    return world_normal;
}

fn _pbr_occlusion_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    cache: MipCache
) -> f32 {
    var occ = 1.0;
    if material.has_occlusion_texture {
        let grads = get_atlas_gradients(material.occlusion_tex_info,
                                        triangle_indices,
                                        attribute_data_offset,
                                        vertex_attribute_stride,
                                        cache);
        let tex = texture_sample_grad_atlas(material.occlusion_tex_info, attribute_uv, grads);
        occ = mix(1.0, tex.r, material.occlusion_strength);
    }
    return occ;
}

fn _pbr_material_emissive_color_grad(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    cache: MipCache
) -> vec3<f32> {
    var e = material.emissive_factor;
    if material.has_emissive_texture {
        let grads = get_atlas_gradients(material.emissive_tex_info,
                                        triangle_indices,
                                        attribute_data_offset,
                                        vertex_attribute_stride,
                                        cache);
        e *= texture_sample_grad_atlas(material.emissive_tex_info, attribute_uv, grads).rgb;
    }
    return e;
}


// --- PBR Material Color ENDS HERE ---
