// 14 * 4 = 64 bytes (added precomputed UV transform for optimization)
struct TextureInfoRaw {
    pixel_offset_x: u32,
    pixel_offset_y: u32,
    width: u32,
    height: u32,
    atlas_layer_index: u32,
    entry_attribute_uv_set_index: u32,
    sampler_index: u32,
    address_mode_u: u32,
    address_mode_v: u32,
    padding: u32,  // Atlas padding in pixels
    uv_offset_x: f32,  // Precomputed UV offset = (texel_offset + 0.5) / atlas_dimensions
    uv_offset_y: f32,
    uv_scale_x: f32,   // Precomputed UV scale = span / atlas_dimensions
    uv_scale_y: f32,
    grad_scale_x: f32,   // Precomputed UV scale = width / atlas_dimensions
    grad_scale_y: f32,
}

struct TextureInfo {
    pixel_offset: vec2<u32>,
    size: vec2<u32>,
    atlas_index: u32,
    layer_index: u32,
    entry_index: u32,
    attribute_uv_set_index: u32,
    sampler_index: u32,
    address_mode_u: u32,
    address_mode_v: u32,
    padding: u32,
    uv_offset: vec2<f32>,  // Precomputed for direct use
    uv_scale: vec2<f32>,   // Precomputed for direct use
    grad_scale: vec2<f32>,   // Precomputed for direct use
}

fn convert_texture_info(raw: TextureInfoRaw) -> TextureInfo {
    return TextureInfo(
        vec2<u32>(raw.pixel_offset_x, raw.pixel_offset_y),
        vec2<u32>(raw.width, raw.height),
        raw.atlas_layer_index & 0xFFFFu,           // atlas_index (16 bits)
        (raw.atlas_layer_index >> 16u) & 0xFFFFu, // layer_index (16 bits)
        raw.entry_attribute_uv_set_index & 0xFFFFu,    // entry_index (16 bits)
        (raw.entry_attribute_uv_set_index >> 16u) & 0xFFFFu, // attribute_uv_index (16 bits)
        raw.sampler_index,
        raw.address_mode_u,
        raw.address_mode_v,
        raw.padding,
        vec2<f32>(raw.uv_offset_x, raw.uv_offset_y),
        vec2<f32>(raw.uv_scale_x, raw.uv_scale_y),
        vec2<f32>(raw.grad_scale_x, raw.grad_scale_y),
    );
}


fn texture_uv(attribute_data_offset: u32, triangle_indices: vec3<u32>, barycentric: vec3<f32>, tex_info: TextureInfo, vertex_attribute_stride: u32) -> vec2<f32> {
    let uv0 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, triangle_indices.x, vertex_attribute_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, triangle_indices.y, vertex_attribute_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, triangle_indices.z, vertex_attribute_stride);

    let interpolated_uv = barycentric.x * uv0 + barycentric.y * uv1 + barycentric.z * uv2;

    return interpolated_uv;
}

fn _texture_uv_per_vertex(attribute_data_offset: u32, set_index: u32, vertex_index: u32, vertex_attribute_stride: u32) -> vec2<f32> {
    // First get to the right vertex, THEN to the right UV set within that vertex
    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride);
    // `uv_sets_index` points to the beginning of TEXCOORD_0 inside the packed stream.
    // Each additional UV set contributes two more floats per vertex.
    let uv_offset = {{ uv_sets_index }}u + (set_index * 2u);
    let index = vertex_start + uv_offset;
    let uv = vec2<f32>(attribute_data[index], attribute_data[index + 1]);

    return uv;
}



// NEW: Sampling with explicit gradients for anisotropic filtering support in compute shaders
fn texture_sample_atlas_grad(info: TextureInfo, attribute_uv: vec2<f32>, uv_derivs: UvDerivs) -> vec4<f32> {
    switch info.atlas_index {
        {% for i in 0..texture_atlas_len %}
            case {{ i }}u: {
                return _texture_sample_atlas_grad(info, atlas_tex_{{ i }}, attribute_uv, uv_derivs);
            }
        {% endfor %}
        default: {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    }
}


fn _texture_sample_atlas_grad(
    info: TextureInfo,
    atlas_tex: texture_2d_array<f32>,
    attribute_uv: vec2<f32>,
    uv_derivs: UvDerivs
) -> vec4<f32> {
    return _texture_sample_atlas_grad_basics(info, atlas_tex, attribute_uv, uv_derivs);
}

fn _texture_sample_atlas_grad_basics(
    info: TextureInfo,
    atlas_tex: texture_2d_array<f32>,
    attribute_uv: vec2<f32>,
    uv_derivs: UvDerivs
) -> vec4<f32> {
    let uv_local  = attribute_uv;           // tile space (continuous)
    let ddx_local = uv_derivs.ddx;          // keep grads continuous
    let ddy_local = uv_derivs.ddy;

    // Manual wrap on COORDS ONLY in tile space
    var uv_wrapped = vec2<f32>(
        apply_address_mode(uv_local.x, info.address_mode_u),
        apply_address_mode(uv_local.y, info.address_mode_v)
    );


    let uv_atlas  = info.uv_offset + uv_wrapped * info.uv_scale;
    let ddx_atlas = ddx_local * info.grad_scale;
    let ddy_atlas = ddy_local * info.grad_scale;

    // TEMP debug: force base LOD
    // return textureSampleLevel(atlas_tex, atlas_sampler_{{ clamp_sampler_index }}, uv_atlas, i32(info.layer_index), 0.0);

    return textureSampleGrad(
        atlas_tex,
        atlas_sampler_{{ clamp_sampler_index }},
        uv_atlas,
        i32(info.layer_index),
        ddx_atlas,
        ddy_atlas
    );
}
fn _texture_sample_atlas_grad_advanced_once_basics_are_fixed(
    info: TextureInfo,
    atlas_tex: texture_2d_array<f32>,
    attribute_uv: vec2<f32>,
    uv_derivs: UvDerivs
) -> vec4<f32> {
    const ATLAS_HAS_GUTTERS : bool = false;

    // Non-atlas: let the sampler do addressing. DO NOT manual wrap.
    if (!uses_atlas(info)) {
        let uv  = info.uv_offset + attribute_uv * info.uv_scale;
        let ddx = uv_derivs.ddx * info.grad_scale;
        let ddy = uv_derivs.ddy * info.grad_scale;

        // TEMP debug: force base LOD if needed
        // return textureSampleLevel(atlas_tex, atlas_sampler_0, uv, i32(info.layer_index), 0.0);
        switch info.sampler_index {
            {% for i in 0..sampler_atlas_len %}
                case {{ i }}u: {
                    return textureSampleGrad(
                        atlas_tex,
                        atlas_sampler_{{ i }},
                        uv,
                        i32(info.layer_index),
                        ddx,
                        ddy,
                    );
                }
            {% endfor %}
            default: {
                return vec4<f32>(0.0, 0.0, 0.0, 0.0);
            }
        }
    }

    // Atlas path
    let uv_local  = attribute_uv;           // tile space (continuous)
    let ddx_local = uv_derivs.ddx;          // keep grads continuous
    let ddy_local = uv_derivs.ddy;

    // Manual wrap on COORDS ONLY in tile space
    var uv_wrapped = vec2<f32>(
        apply_address_mode(uv_local.x, info.address_mode_u),
        apply_address_mode(uv_local.y, info.address_mode_v)
    );

    // Map into atlas. Two modes:
    if (ATLAS_HAS_GUTTERS) {
        // ----- GUTTER MODE (enable after your builder adds per-mip padding) -----
        let atlas_dims = vec2<f32>(textureDimensions(atlas_tex, 0u));
        let pad_uv = vec2<f32>(f32(info.padding)) / atlas_dims;

        // Variant A: uv_offset/uv_scale ALREADY include padding (most common)
        let linear = max(info.uv_scale - 2.0 * pad_uv, vec2<f32>(0.0));
        let uv_atlas  = info.uv_offset + (uv_wrapped * linear + pad_uv);
        let ddx_atlas = ddx_local * linear;
        let ddy_atlas = ddy_local * linear;

        // TEMP debug: force base LOD
        // return textureSampleLevel(atlas_tex, atlas_sampler_{{ clamp_sampler_index }}, uv_atlas, i32(info.layer_index), 0.0);

        return textureSampleGrad(
            atlas_tex,
            atlas_sampler_{{ clamp_sampler_index }},
            uv_atlas,
            i32(info.layer_index),
            ddx_atlas,
            ddy_atlas
        );

    } else {
        // ----- NO-GUTTER FALLBACK (matches your original visual behavior) -----
        // WARNING: with clamp sampler this can show seams at tile edges.
        // If you must avoid seams without rebuilding the atlas, there is no perfect solution.
        let uv_atlas  = info.uv_offset + uv_wrapped * info.uv_scale;
        let ddx_atlas = ddx_local * info.grad_scale;
        let ddy_atlas = ddy_local * info.grad_scale;

        // TEMP debug: force base LOD
        // return textureSampleLevel(atlas_tex, atlas_sampler_{{ clamp_sampler_index }}, uv_atlas, i32(info.layer_index), 0.0);

        return textureSampleGrad(
            atlas_tex,
            atlas_sampler_{{ clamp_sampler_index }},
            uv_atlas,
            i32(info.layer_index),
            ddx_atlas,
            ddy_atlas
        );
    }
}


const ADDRESS_MODE_CLAMP_TO_EDGE : u32 = 0u;
const ADDRESS_MODE_REPEAT        : u32 = 1u;
const ADDRESS_MODE_MIRROR_REPEAT : u32 = 2u;

fn uses_atlas(info: TextureInfo) -> bool {
    return any(info.uv_scale  != vec2<f32>(1.0)) ||
           any(info.uv_offset != vec2<f32>(0.0));
}

fn wrap_mirror(coord: f32) -> f32 {
    let floored = floor(coord);
    let frac    = coord - floored;
    let is_odd  = (i32(floored) & 1) != 0;
    return select(frac, 1.0 - frac, is_odd);
}

// Manual per-axis address mode for coords only
fn apply_address_mode(coord: f32, mode: u32) -> f32 {
    switch (mode) {
        case ADDRESS_MODE_CLAMP_TO_EDGE: { return clamp(coord, 0.0, 1.0); }
        case ADDRESS_MODE_MIRROR_REPEAT: { return wrap_mirror(coord); }
        case ADDRESS_MODE_REPEAT:        { return fract(coord); }
        default: { return 0.0; }
    }
}


// Sampling helpers for the mega-texture atlas. Every fetch receives an explicit LOD so the compute
// pass can emulate hardware derivative selection.
fn texture_sample_atlas_no_mips(info: TextureInfo, attribute_uv: vec2<f32>) -> vec4<f32> {
    switch info.atlas_index {
        {% for i in 0..texture_atlas_len %}
            case {{ i }}u: {
                return _texture_sample_atlas_no_mips(info, atlas_tex_{{ i }}, attribute_uv);
            }
        {% endfor %}
        default: {
            // If we somehow reference an out-of-range sampler (should not happen), return black to
            // avoid propagating NaNs that could poison later colour math.
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    }
}

fn _texture_sample_atlas_no_mips(
    info: TextureInfo,
    atlas_tex: texture_2d_array<f32>,
    attribute_uv: vec2<f32>,
) -> vec4<f32> {
    let wrapped_uv = vec2<f32>(
        apply_address_mode(attribute_uv.x, info.address_mode_u),
        apply_address_mode(attribute_uv.y, info.address_mode_v),
    );

    // Use precomputed UV transform (eliminates textureDimensions() call and conversions)
    let uv = info.uv_offset + wrapped_uv * info.uv_scale;

    switch info.sampler_index {
        {% for i in 0..sampler_atlas_len %}
            case {{ i }}u: {
                return textureSampleLevel(
                    atlas_tex,
                    atlas_sampler_{{ i }},
                    uv,
                    i32(info.layer_index),
                    0
                );
            }
        {% endfor %}
        default: {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    }
}
