
{% if mipmap.is_gradient() %}
struct PbrMaterialGradients {
    base_color: UvDerivs,
    metallic_roughness: UvDerivs,
    normal: UvDerivs,
    occlusion: UvDerivs,
    emissive: UvDerivs,
    specular: UvDerivs,
    specular_color: UvDerivs,
    transmission: UvDerivs,
    volume_thickness: UvDerivs,
    // KHR_materials_clearcoat
    clearcoat: UvDerivs,
    clearcoat_roughness: UvDerivs,
    clearcoat_normal: UvDerivs,
    // KHR_materials_sheen
    sheen_color: UvDerivs,
    sheen_roughness: UvDerivs,
}
{% endif %}

// Main PBR material color function - samples all textures and computes final material properties
// Returns PbrMaterialColor with perturbed normal (use material_color.normal for lighting!)
fn pbr_get_material_color{{ mipmap.suffix() }}(
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    triangle_index: u32,
    material: PbrMaterial,
    barycentric: vec3<f32>,
    vertex_attribute_stride: u32,
    uv_sets_index: u32,
    {% if mipmap.is_gradient() %}gradients: PbrMaterialGradients,{% endif %}
    geometry_tbn: TBN,
) -> PbrMaterialColor {
    // Load extension data on-demand from indices
    let emissive_strength = pbr_material_load_emissive_strength(material.emissive_strength_index);
    let ior = pbr_material_load_ior(material.ior_index);
    let specular = pbr_material_load_specular(material.specular_index);
    let transmission = pbr_material_load_transmission(material.transmission_index);
    let volume = pbr_material_load_volume(material.volume_index);
    let clearcoat = pbr_material_load_clearcoat(material.clearcoat_index);
    let sheen = pbr_material_load_sheen(material.sheen_index);

    var base = _pbr_material_base_color{{ mipmap.suffix() }}(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.base_color_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.base_color,{% endif %}
    );

    {%- match color_sets %}
        {% when Some with (color_sets) %}
            let vertex_color_info = pbr_material_load_vertex_color_info(material.vertex_color_info_index);
            base *= vertex_color(
                attribute_data_offset,
                triangle_indices,
                barycentric,
                vertex_color_info,
                vertex_attribute_stride,
            );
        {% when _ %}
    {% endmatch %}

    let metallic_roughness = _pbr_material_metallic_roughness_color{{ mipmap.suffix() }}(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.metallic_roughness_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.metallic_roughness,{% endif %}
    );

    let normal = _pbr_normal_color{{ mipmap.suffix() }}(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.normal_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.normal,{% endif %}
        geometry_tbn,
    );

    let occlusion = _pbr_occlusion_color{{ mipmap.suffix() }}(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.occlusion_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.occlusion,{% endif %}
    );

    let emissive = _pbr_material_emissive_color{{ mipmap.suffix() }}(
        material,
        emissive_strength,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.emissive_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.emissive,{% endif %}
    );

    let specular_factor = _pbr_specular{{ mipmap.suffix() }}(
        specular,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            specular.tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.specular,{% endif %}
    );

    let specular_color_factor = _pbr_specular_color{{ mipmap.suffix() }}(
        specular,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            specular.color_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.specular_color,{% endif %}
    );

    let transmission_factor = _pbr_transmission{{ mipmap.suffix() }}(
        transmission,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            transmission.tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.transmission,{% endif %}
    );

    let volume_thickness = _pbr_volume_thickness{{ mipmap.suffix() }}(
        volume,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            volume.thickness_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.volume_thickness,{% endif %}
    );

    // Clearcoat sampling
    let clearcoat_factor = _pbr_clearcoat{{ mipmap.suffix() }}(
        clearcoat,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            clearcoat.tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.clearcoat,{% endif %}
    );

    let clearcoat_roughness_factor = _pbr_clearcoat_roughness{{ mipmap.suffix() }}(
        clearcoat,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            clearcoat.roughness_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.clearcoat_roughness,{% endif %}
    );

    let clearcoat_normal_value = _pbr_clearcoat_normal{{ mipmap.suffix() }}(
        clearcoat,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            clearcoat.normal_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.clearcoat_normal,{% endif %}
        geometry_tbn,
    );

    // Sheen sampling
    let sheen_color_factor = _pbr_sheen_color{{ mipmap.suffix() }}(
        sheen,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            sheen.color_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.sheen_color,{% endif %}
    );

    let sheen_roughness_factor = _pbr_sheen_roughness{{ mipmap.suffix() }}(
        sheen,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            sheen.roughness_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        ),
        {% if mipmap.is_gradient() %}gradients.sheen_roughness,{% endif %}
    );

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
        volume.attenuation_color,
        // Clearcoat
        clearcoat_factor,
        clearcoat_roughness_factor,
        clearcoat_normal_value,
        // Sheen
        sheen_color_factor,
        sheen_roughness_factor,
    );
}

// Base Color
fn _pbr_material_base_color{{ mipmap.suffix() }}(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> vec4<f32> {
    var color = material.base_color_factor;
    if material.base_color_tex_info.exists {
        let tex_sample = {{ mipmap.sample_fn() }}(material.base_color_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %});
        color *= tex_sample;
    }
    // compute pass only deals with fully opaque
    // mask and blend are handled in the fragment shader
    color.a = 1.0;
    return color;
}

// Metallic-Roughness
fn _pbr_material_metallic_roughness_color{{ mipmap.suffix() }}(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> vec2<f32> {
    var color = vec2<f32>(material.metallic_factor, material.roughness_factor);
    if material.metallic_roughness_tex_info.exists {
        let tex = {{ mipmap.sample_fn() }}(material.metallic_roughness_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %});
        // glTF uses B channel for metallic, G channel for roughness
        color *= vec2<f32>(tex.b, tex.g);
    }
    return color;
}

// Normal mapping - transforms normal texture from tangent to world space using geometry TBN
// The TBN is passed from the geometry pass (already interpolated and transformed)
fn _pbr_normal_color{{ mipmap.suffix() }}(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
    geometry_tbn: TBN,
) -> vec3<f32> {
    if !material.normal_tex_info.exists {
        return geometry_tbn.N;
    }

    // Sample normal map and unpack from [0,1] to [-1,1] range
    let tex = {{ mipmap.sample_fn() }}(material.normal_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %});
    let tangent_normal = vec3<f32>(
        (tex.r * 2.0 - 1.0) * material.normal_scale,
        (tex.g * 2.0 - 1.0) * material.normal_scale,
        tex.b * 2.0 - 1.0,
    );

    // Transform the tangent-space normal to world space using the TBN matrix from geometry pass
    let tbn_matrix = mat3x3<f32>(geometry_tbn.T, geometry_tbn.B, geometry_tbn.N);
    return normalize(tbn_matrix * tangent_normal);
}

// Occlusion
fn _pbr_occlusion_color{{ mipmap.suffix() }}(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> f32 {
    var occlusion = 1.0;
    if material.occlusion_tex_info.exists {
        let tex = {{ mipmap.sample_fn() }}(material.occlusion_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %});
        occlusion = mix(1.0, tex.r, material.occlusion_strength);
    }
    return occlusion;
}

// Emissive
fn _pbr_material_emissive_color{{ mipmap.suffix() }}(
    material: PbrMaterial,
    emissive_strength: f32,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> vec3<f32> {
    var color = material.emissive_factor;
    if material.emissive_tex_info.exists {
        color *= {{ mipmap.sample_fn() }}(material.emissive_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %}).rgb;
    }
    color *= emissive_strength;
    return color;
}

// Specular factor
fn _pbr_specular{{ mipmap.suffix() }}(
    specular: PbrSpecular,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> f32 {
    var factor = specular.factor;
    if specular.tex_info.exists {
        factor *= {{ mipmap.sample_fn() }}(specular.tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %}).a;
    }
    return factor;
}

// Specular color
fn _pbr_specular_color{{ mipmap.suffix() }}(
    specular: PbrSpecular,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> vec3<f32> {
    var color = specular.color_factor;
    if specular.color_tex_info.exists {
        color *= {{ mipmap.sample_fn() }}(specular.color_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %}).rgb;
    }
    return color;
}

// Transmission
fn _pbr_transmission{{ mipmap.suffix() }}(
    transmission: PbrTransmission,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> f32 {
    // Early exit: if no texture and factor is 0, skip entirely
    if (!transmission.tex_info.exists && transmission.factor == 0.0) {
        return 0.0;
    }
    var factor = transmission.factor;
    if transmission.tex_info.exists {
        factor *= {{ mipmap.sample_fn() }}(transmission.tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %}).r;
    }
    return factor;
}

// Volume thickness
fn _pbr_volume_thickness{{ mipmap.suffix() }}(
    volume: PbrVolume,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> f32 {
    // Early exit: no volume if thickness is 0 and no texture
    if (!volume.thickness_tex_info.exists && volume.thickness_factor == 0.0) {
        return 0.0;
    }
    var thickness = volume.thickness_factor;
    if volume.thickness_tex_info.exists {
        // Volume thickness is stored in the G channel per glTF spec
        thickness *= {{ mipmap.sample_fn() }}(volume.thickness_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %}).g;
    }
    return thickness;
}

// ============================================================================
// Clearcoat (KHR_materials_clearcoat)
// ============================================================================

// Clearcoat intensity factor (R channel)
fn _pbr_clearcoat{{ mipmap.suffix() }}(
    clearcoat: PbrClearcoat,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> f32 {
    // Early exit: no clearcoat if factor is 0 and no texture
    if (!clearcoat.tex_info.exists && clearcoat.factor == 0.0) {
        return 0.0;
    }
    var factor = clearcoat.factor;
    if clearcoat.tex_info.exists {
        factor *= {{ mipmap.sample_fn() }}(clearcoat.tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %}).r;
    }
    return factor;
}

// Clearcoat roughness (G channel)
fn _pbr_clearcoat_roughness{{ mipmap.suffix() }}(
    clearcoat: PbrClearcoat,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> f32 {
    var roughness = clearcoat.roughness_factor;
    if clearcoat.roughness_tex_info.exists {
        roughness *= {{ mipmap.sample_fn() }}(clearcoat.roughness_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %}).g;
    }
    return roughness;
}

// Clearcoat normal - transforms clearcoat normal texture from tangent to world space using geometry TBN
fn _pbr_clearcoat_normal{{ mipmap.suffix() }}(
    clearcoat: PbrClearcoat,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
    geometry_tbn: TBN,
) -> vec3<f32> {
    // If no clearcoat normal texture, use geometry normal
    if !clearcoat.normal_tex_info.exists {
        return geometry_tbn.N;
    }

    // Sample clearcoat normal map and unpack from [0,1] to [-1,1] range
    let tex = {{ mipmap.sample_fn() }}(clearcoat.normal_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %});
    let tangent_normal = vec3<f32>(
        (tex.r * 2.0 - 1.0) * clearcoat.normal_scale,
        (tex.g * 2.0 - 1.0) * clearcoat.normal_scale,
        tex.b * 2.0 - 1.0,
    );

    // Transform the tangent-space normal to world space using the TBN matrix from geometry pass
    let tbn_matrix = mat3x3<f32>(geometry_tbn.T, geometry_tbn.B, geometry_tbn.N);
    return normalize(tbn_matrix * tangent_normal);
}

// ============================================================================
// Sheen (KHR_materials_sheen)
// ============================================================================

// Sheen color (RGB)
fn _pbr_sheen_color{{ mipmap.suffix() }}(
    sheen: PbrSheen,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> vec3<f32> {
    var color = sheen.color_factor;
    if sheen.color_tex_info.exists {
        color *= {{ mipmap.sample_fn() }}(sheen.color_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %}).rgb;
    }
    return color;
}

// Sheen roughness (A channel)
fn _pbr_sheen_roughness{{ mipmap.suffix() }}(
    sheen: PbrSheen,
    attribute_uv: vec2<f32>,
    {% if mipmap.is_gradient() %}uv_derivs: UvDerivs,{% endif %}
) -> f32 {
    var roughness = sheen.roughness_factor;
    if sheen.roughness_tex_info.exists {
        roughness *= {{ mipmap.sample_fn() }}(sheen.roughness_tex_info, attribute_uv{% if mipmap.is_gradient() %}, uv_derivs{% endif %}).a;
    }
    return roughness;
}

// ============================================================================
// Unlit Material Color Computation
// ============================================================================

// Compute unlit material color
fn compute_unlit_material_color(
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    material: UnlitMaterial,
    barycentric: vec3<f32>,
    vertex_attribute_stride: u32,
    uv_sets_index: u32,
    {% if mipmap.is_gradient() %}
    bary_derivs: vec4<f32>,
    world_normal: vec3<f32>,
    view_matrix: mat4x4<f32>,
    {% endif %}
) -> UnlitMaterialColor {
    // Compute base color
    var base = material.base_color_factor;
    if material.base_color_tex_info.exists {
        let uv = texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.base_color_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        );
        {% if mipmap.is_gradient() %}
        let gradients = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset,
            vertex_attribute_stride,
            uv_sets_index,
            material.base_color_tex_info,
            world_normal,
            view_matrix
        );
        base *= texture_pool_sample_grad(material.base_color_tex_info, uv, gradients);
        {% else %}
        base *= texture_pool_sample_no_mips(material.base_color_tex_info, uv);
        {% endif %}
    }

    // Compute emissive
    var emissive = material.emissive_factor;
    if material.emissive_tex_info.exists {
        let uv = texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.emissive_tex_info,
            vertex_attribute_stride,
            uv_sets_index,
        );
        {% if mipmap.is_gradient() %}
        let gradients = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset,
            vertex_attribute_stride,
            uv_sets_index,
            material.emissive_tex_info,
            world_normal,
            view_matrix
        );
        emissive *= texture_pool_sample_grad(material.emissive_tex_info, uv, gradients).rgb;
        {% else %}
        emissive *= texture_pool_sample_no_mips(material.emissive_tex_info, uv).rgb;
        {% endif %}
    }

    // Opaque pass forces alpha to 1.0
    base.a = 1.0;

    return UnlitMaterialColor(base, emissive);
}

// ============================================================================
// Tangent Helpers
// ============================================================================

// Interpolate tangent vectors across a triangle using barycentric coordinates
fn get_vertex_tangent(
    attribute_data_offset: u32,
    triangle_indices: vec3<u32>,
    barycentric: vec3<f32>,
    vertex_attribute_stride: u32,
) -> vec4<f32> {
    let t0 = _get_vertex_tangent(attribute_data_offset, triangle_indices.x, vertex_attribute_stride);
    let t1 = _get_vertex_tangent(attribute_data_offset, triangle_indices.y, vertex_attribute_stride);
    let t2 = _get_vertex_tangent(attribute_data_offset, triangle_indices.z, vertex_attribute_stride);
    return barycentric.x * t0 + barycentric.y * t1 + barycentric.z * t2;
}

// Read tangent from packed attribute buffer
// Attribute layout per vertex: [normal.xyz (3 floats), tangent.xyzw (4 floats), ...]
fn _get_vertex_tangent(
    attribute_data_offset: u32,
    vertex_index: u32,
    vertex_attribute_stride: u32,
) -> vec4<f32> {
    if (vertex_attribute_stride < 7u) {
        // No tangent data available (stride < normal(3) + tangent(4))
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride);
    let base = vertex_start + 3u; // tangents follow normals (3 float offset)

    return vec4<f32>(
        attribute_data[base],
        attribute_data[base + 1u],
        attribute_data[base + 2u],
        attribute_data[base + 3u],  // w component = handedness sign (±1)
    );
}
