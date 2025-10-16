// Nudges the sampled mip slightly sharper to counter our finite-difference bias.
const MIPMAP_GLOBAL_LOD_BIAS: f32 = -0.5;

// Matches `MegaTexture::new` padding. Keep both in sync.
const MIPMAP_ATLAS_PADDING: f32 = 8.0;
const MIPMAP_CLAMP_EPSILON: f32 = 1e-4;


// Derive texture LODs by comparing UVs between neighbouring pixels. This approximates the
// derivatives hardware would have given us in a fragment shader and keeps mip selection
// consistent even though we deferred shading to a compute pass.
// Approximate mip selection via neighbour UVs. We wrap the intermediate UVs using the same
// addressing logic as the sampler so clamp-to-edge regions don't artificially inflate the
// gradient (which would otherwise generate thin seams near borders).
//
fn pbr_get_mipmap_levels(
    pbr_material: PbrMaterial,
    coords: vec2<i32>,
    triangle_index: u32,
    barycentric: vec3<f32>,
    attribute_indices_offset: u32,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    screen_dims_i32: vec2<i32>,
) -> PbrMaterialMipLevels {
    let triangle_indices_current = get_triangle_indices(attribute_indices_offset, triangle_index);

    let base_color_lod = compute_texture_mipmap_lod(
        pbr_material.base_color_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_base_color_texture,
    );

    let metallic_roughness_lod = compute_texture_mipmap_lod(
        pbr_material.metallic_roughness_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_metallic_roughness_texture,
    );

    let normal_lod = compute_texture_mipmap_lod(
        pbr_material.normal_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_normal_texture,
    );

    let occlusion_lod = compute_texture_mipmap_lod(
        pbr_material.occlusion_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_occlusion_texture,
    );

    let emissive_lod = compute_texture_mipmap_lod(
        pbr_material.emissive_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_emissive_texture,
    );

    return PbrMaterialMipLevels(
        base_color_lod,
        metallic_roughness_lod,
        normal_lod,
        occlusion_lod,
        emissive_lod,
    );
}


fn compute_texture_mipmap_lod(
    tex_info: TextureInfo,
    coords: vec2<i32>,
    triangle_indices_current: vec3<u32>,
    triangle_index: u32,
    barycentric: vec3<f32>,
    attribute_indices_offset: u32,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    screen_dims: vec2<i32>,
    texture_enabled: bool,
) -> f32 {
    if (!texture_enabled) {
        return 0.0;
    }

    let uv_center = texture_uv(
        attribute_data_offset,
        triangle_indices_current,
        barycentric,
        tex_info,
        vertex_attribute_stride,
    );

    let uv_right = sample_neighbor_uv(
        coords,
        vec2<i32>(1, 0),
        tex_info,
        triangle_index,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims,
        uv_center,
    );

    let uv_up = sample_neighbor_uv(
        coords,
        vec2<i32>(0, 1),
        tex_info,
        triangle_index,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims,
        uv_center,
    );

    let uv_center_wrapped = vec2<f32>(
        apply_address_mode(uv_center.x, tex_info.address_mode_u),
        apply_address_mode(uv_center.y, tex_info.address_mode_v),
    );
    let uv_right_wrapped = vec2<f32>(
        apply_address_mode(uv_right.x, tex_info.address_mode_u),
        apply_address_mode(uv_right.y, tex_info.address_mode_v),
    );
    let uv_up_wrapped = vec2<f32>(
        apply_address_mode(uv_up.x, tex_info.address_mode_u),
        apply_address_mode(uv_up.y, tex_info.address_mode_v),
    );
    let tex_scale = vec2<f32>(f32(tex_info.size.x), f32(tex_info.size.y));
    let delta_x = wrap_delta_vec(
        uv_right_wrapped - uv_center_wrapped,
        tex_info.address_mode_u,
        tex_info.address_mode_v,
    );
    let delta_y = wrap_delta_vec(
        uv_up_wrapped - uv_center_wrapped,
        tex_info.address_mode_u,
        tex_info.address_mode_v,
    );
    let uv_dx = delta_x * tex_scale;
    let uv_dy = delta_y * tex_scale;
    let gradient = max(length(uv_dx), length(uv_dy));
    let lod = log2(max(gradient, 1e-6));
    let max_mip = log2(max(f32(tex_info.size.x), f32(tex_info.size.y)));

    var clamped_lod = clamp(lod, 0.0, max_mip);
    let clamp_u = tex_info.address_mode_u == ADDRESS_MODE_CLAMP_TO_EDGE;
    let clamp_v = tex_info.address_mode_v == ADDRESS_MODE_CLAMP_TO_EDGE;
    let oob_u = clamp_u && (uv_center.x < -MIPMAP_CLAMP_EPSILON || uv_center.x > 1.0 + MIPMAP_CLAMP_EPSILON);
    let oob_v = clamp_v && (uv_center.y < -MIPMAP_CLAMP_EPSILON || uv_center.y > 1.0 + MIPMAP_CLAMP_EPSILON);
    if (oob_u || oob_v) {
        let max_clamp_lod = log2(max(MIPMAP_ATLAS_PADDING - 1.0, 1.0));
        clamped_lod = min(clamped_lod, max_clamp_lod);
    }

    clamped_lod = clamp(clamped_lod + MIPMAP_GLOBAL_LOD_BIAS, 0.0, max_mip);

    return clamped_lod;
}

fn sample_neighbor_uv(
    coords: vec2<i32>,
    offset: vec2<i32>,
    tex_info: TextureInfo,
    triangle_index: u32,
    attribute_indices_offset: u32,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    screen_dims: vec2<i32>,
    fallback_uv: vec2<f32>,
) -> vec2<f32> {
    let neighbor = coords + offset;
    if (neighbor.x < 0 || neighbor.y < 0 || neighbor.x >= screen_dims.x || neighbor.y >= screen_dims.y) {
        return fallback_uv;
    }

    let neighbor_visibility = textureLoad(visibility_data_tex, neighbor, 0);
    let neighbor_triangle_index = bitcast<u32>(neighbor_visibility.x);
    if (neighbor_triangle_index == f32_max || neighbor_triangle_index != triangle_index) {
        return fallback_uv;
    }

    let barycentric = vec3<f32>(
        neighbor_visibility.z,
        neighbor_visibility.w,
        1.0 - neighbor_visibility.z - neighbor_visibility.w,
    );
    let neighbor_triangle_indices =
        get_triangle_indices(attribute_indices_offset, neighbor_triangle_index);

    return texture_uv(
        attribute_data_offset,
        neighbor_triangle_indices,
        barycentric,
        tex_info,
        vertex_attribute_stride,
    );
}

fn wrap_delta(delta: f32, mode: u32) -> f32 {
    if (mode == ADDRESS_MODE_REPEAT) {
        return delta - round(delta);
    }

    if (mode == ADDRESS_MODE_MIRROR_REPEAT) {
        let wrapped = delta - round(delta * 0.5) * 2.0;
        return wrapped;
    }

    return delta;
}

fn wrap_delta_vec(delta: vec2<f32>, mode_u: u32, mode_v: u32) -> vec2<f32> {
    return vec2<f32>(
        wrap_delta(delta.x, mode_u),
        wrap_delta(delta.y, mode_v),
    );
}
