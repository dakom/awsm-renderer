// ============================================================================
// mipmap.wgsl — Analytic mip selection in compute (atlas-aware, WGSL-safe)
// ============================================================================

const MIPMAP_GLOBAL_LOD_BIAS : f32 = -0.5;
const MIPMAP_CLAMP_EPSILON   : f32 = 1e-4;
const MIPMAP_MIN_DET         : f32 = 1e-6;
const MIPMAP_ATLAS_PADDING   : f32 = 8.0; // texels of content padding per sub-rect

// ─────────────────────────────────────────────────────────────────────────────
// Shared structs
// ─────────────────────────────────────────────────────────────────────────────
struct PlaneCoefficients {
    base : f32,
    dx   : f32,
    dy   : f32,
    valid: bool,
}

struct MipCache {
    valid        : bool,
    plane_inv_w  : PlaneCoefficients,
    s0           : vec2<f32>,
    s1           : vec2<f32>,
    s2           : vec2<f32>,
    invw0        : f32,
    invw1        : f32,
    invw2        : f32,
    pixel_center : vec2<f32>,
}

struct UvDerivs {
    dudx : f32,
    dudy : f32,
    dvdx : f32,
    dvdy : f32,
}

struct Grad2 {
    dudx_dvdx : vec2<f32>,
    dudy_dvdy : vec2<f32>,
}

struct PbrMaterialMipLevels {
    base_color         : f32,
    metallic_roughness : f32,
    normal             : f32,
    occlusion          : f32,
    emissive           : f32,
}

struct AtlasInfo {
    dims     : vec2<f32>, // textureDimensions(atlas, 0u)
    levels_f : f32,       // f32(textureNumLevels(atlas))
    valid    : bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

fn pbr_get_mipmap_levels(
    cache: MipCache,
    screen_dims: vec2<f32>,
    material: PbrMaterial,
    triangle_indices: vec3<u32>,
    barycentric: vec3<f32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
) -> PbrMaterialMipLevels {

    var out : PbrMaterialMipLevels;

    if (material.has_base_color_texture) {
        out.base_color = compute_texture_lod_atlas_space(
            material.base_color_tex_info,
            triangle_indices, barycentric,
            attribute_data_offset, vertex_attribute_stride,
            cache, screen_dims
        );
    } else { out.base_color = 0.0; }

    if (material.has_metallic_roughness_texture) {
        out.metallic_roughness = compute_texture_lod_atlas_space(
            material.metallic_roughness_tex_info,
            triangle_indices, barycentric,
            attribute_data_offset, vertex_attribute_stride,
            cache, screen_dims
        );
    } else { out.metallic_roughness = 0.0; }

    if (material.has_normal_texture) {
        out.normal = compute_texture_lod_atlas_space(
            material.normal_tex_info,
            triangle_indices, barycentric,
            attribute_data_offset, vertex_attribute_stride,
            cache, screen_dims
        );
    } else { out.normal = 0.0; }

    if (material.has_occlusion_texture) {
        out.occlusion = compute_texture_lod_atlas_space(
            material.occlusion_tex_info,
            triangle_indices, barycentric,
            attribute_data_offset, vertex_attribute_stride,
            cache, screen_dims
        );
    } else { out.occlusion = 0.0; }

    if (material.has_emissive_texture) {
        out.emissive = compute_texture_lod_atlas_space(
            material.emissive_tex_info,
            triangle_indices, barycentric,
            attribute_data_offset, vertex_attribute_stride,
            cache, screen_dims
        );
    } else { out.emissive = 0.0; }

    return out;
}

// Optional gradients for textureSampleGrad()
fn get_texture_gradients(
    tex_info: TextureInfo,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    cache: MipCache
) -> Grad2 {
    var g = Grad2(vec2<f32>(0.0), vec2<f32>(0.0));
    if (!cache.valid) { return g; }

    let d = uv_derivs_local(tex_info, triangle_indices, attribute_data_offset, vertex_attribute_stride, cache);
    if (d.dudx != 0.0 || d.dudy != 0.0 || d.dvdx != 0.0 || d.dvdy != 0.0) {
        g.dudx_dvdx = vec2<f32>(d.dudx, d.dvdx);
        g.dudy_dvdy = vec2<f32>(d.dudy, d.dvdy);
    }
    return g;
}

// Map local [0,1] UV to atlas UV (no sampling).
fn atlas_transform(info: TextureInfo, attribute_uv: vec2<f32>, atlas_dims: vec2<f32>) -> vec2<f32> {
    let wrapped = vec2<f32>(
        apply_address_mode(attribute_uv.x, info.address_mode_u),
        apply_address_mode(attribute_uv.y, info.address_mode_v)
    );

    let texel_offset = vec2<f32>(info.pixel_offset);
    let texel_size   = vec2<f32>(info.size);
    let span = max(texel_size - vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 0.0));
    let texel_coords = texel_offset + wrapped * span + vec2<f32>(0.5, 0.5);
    return texel_coords / atlas_dims;
}

// ─────────────────────────────────────────────────────────────────────────────
// Implementation
// ─────────────────────────────────────────────────────────────────────────────


fn build_mip_cache_with_barycentric(
    projected_vertices: ProjectedVertices,
    pixel_center: vec2<f32>,
    // tri: vec3<u32>,
    // barycentric: vec3<f32>,
    // visibility_data_offset: u32,
    // triangle_index: u32,
    // vertex_stride: u32,
    // model_transform: mat4x4<f32>,
) -> MipCache {
    let pr0 = projected_vertices.p0;
    let pr1 = projected_vertices.p1;
    let pr2 = projected_vertices.p2;


    if (!(pr0.valid && pr1.valid && pr2.valid)) {
        return MipCache(false,
                        PlaneCoefficients(0.0, 0.0, 0.0, false),
                        vec2<f32>(0.0), vec2<f32>(0.0), vec2<f32>(0.0),
                        0.0, 0.0, 0.0,
                        pixel_center);
    }

    let plane_inv_w = fit_plane(pr0.screen, pr1.screen, pr2.screen,
                                pr0.inv_w,   pr1.inv_w,   pr2.inv_w);

    return MipCache(true,
                    plane_inv_w,
                    pr0.screen, pr1.screen, pr2.screen,
                    pr0.inv_w,  pr1.inv_w,  pr2.inv_w,
                    pixel_center);
}



fn uv_derivs_local(
    tex_info: TextureInfo,
    tri: vec3<u32>,
    attribute_data_offset: u32,
    vertex_stride: u32,
    cache: MipCache
) -> UvDerivs {
    // NOTE: Don't return zero derivatives! This causes a discontinuity when cache becomes invalid.
    // Instead, we'll compute what we can and let the caller handle any remaining edge cases.
    var out = UvDerivs(0.0, 0.0, 0.0, 0.0);

    // If cache is invalid, we can't compute screen-space derivatives
    // Caller should use uv_derivs_barycentric as fallback
    if (!cache.valid) { return out; }

    let uv0 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, tri.x, vertex_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, tri.y, vertex_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, tri.z, vertex_stride);

    // Screen space positions
    let s0 = cache.s0;
    let s1 = cache.s1;
    let s2 = cache.s2;

    // Edge vectors in screen space
    let e01_screen = s1 - s0;
    let e02_screen = s2 - s0;

    // Edge vectors in UV space - CRITICAL: wrap deltas to handle repeating textures
    // When UVs repeat (e.g., 0.9 to 0.1), the delta should be 0.2, not 0.8
    var e01_uv = uv1 - uv0;
    var e02_uv = uv2 - uv0;

    // Only wrap UV deltas for repeating address modes
    // CLAMP_TO_EDGE shouldn't wrap
    if (tex_info.address_mode_u == ADDRESS_MODE_REPEAT || tex_info.address_mode_u == ADDRESS_MODE_MIRROR_REPEAT) {
        e01_uv.x = e01_uv.x - round(e01_uv.x);
        e02_uv.x = e02_uv.x - round(e02_uv.x);
    }
    if (tex_info.address_mode_v == ADDRESS_MODE_REPEAT || tex_info.address_mode_v == ADDRESS_MODE_MIRROR_REPEAT) {
        e01_uv.y = e01_uv.y - round(e01_uv.y);
        e02_uv.y = e02_uv.y - round(e02_uv.y);
    }

    // Compute the 2x2 linear system to get dUV/dScreen
    // [du/dx  du/dy] = [e01_uv.x  e02_uv.x] * [e01_screen.x  e01_screen.y]^-1
    // [dv/dx  dv/dy]   [e01_uv.y  e02_uv.y]   [e02_screen.x  e02_screen.y]

    let det = e01_screen.x * e02_screen.y - e01_screen.y * e02_screen.x;

    // If determinant is too small, triangle is degenerate in screen space
    // Return zero derivatives - caller will use fallback
    if (abs(det) < MIPMAP_MIN_DET) {
        return out;
    }

    let inv_det = 1.0 / det;

    // Inverse of screen edge matrix
    let inv_s_00 = e02_screen.y * inv_det;
    let inv_s_01 = -e01_screen.y * inv_det;
    let inv_s_10 = -e02_screen.x * inv_det;
    let inv_s_11 = e01_screen.x * inv_det;

    // Compute UV derivatives
    let dudx = e01_uv.x * inv_s_00 + e02_uv.x * inv_s_10;
    let dudy = e01_uv.x * inv_s_01 + e02_uv.x * inv_s_11;
    let dvdx = e01_uv.y * inv_s_00 + e02_uv.y * inv_s_10;
    let dvdy = e01_uv.y * inv_s_01 + e02_uv.y * inv_s_11;

    return UvDerivs(dudx, dudy, dvdx, dvdy);
}

fn uv_derivs_barycentric(
    tex_info: TextureInfo,
    tri: vec3<u32>,
    barycentric: vec3<f32>,
    attribute_data_offset: u32,
    vertex_stride: u32,
    screen_dims: vec2<f32>
) -> UvDerivs {
    // Get UV coordinates at triangle vertices
    let uv0 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, tri.x, vertex_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, tri.y, vertex_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, tri.z, vertex_stride);

    // Mathematical approach: UV = w0*uv0 + w1*uv1 + w2*uv2 where w = barycentric coords
    // So dUV/dx = (dw0/dx)*uv0 + (dw1/dx)*uv1 + (dw2/dx)*uv2
    // We need to estimate the derivatives of barycentric coordinates w.r.t. screen space

    // Estimate triangle scale in screen space using barycentric coordinate gradients
    // The determinant of barycentric coordinates gives us information about triangle area
    let bary_det = abs(barycentric.x * barycentric.y - barycentric.x * barycentric.z - barycentric.y * barycentric.z);
    let triangle_area_inv = max(1.0 / max(bary_det, 0.001), 1.0); // Inverse area estimate

    // Estimate screen-space derivatives of barycentric coordinates
    // This assumes roughly uniform triangle size distribution
    let base_bary_deriv = sqrt(triangle_area_inv) * 0.01; // Base derivative scale
    let pixel_scale = 2.0 / min(screen_dims.x, screen_dims.y); // Normalize by screen size
    let bary_derivative_scale = base_bary_deriv * pixel_scale;

    // Clamp to reasonable bounds to avoid extreme values
    let clamped_bary_deriv = clamp(bary_derivative_scale, 0.0001, 0.1);

    // Approximate barycentric derivatives (simplified 2D finite difference approximation)
    // Assume barycentric coordinates change roughly uniformly across triangle
    let dbary0_dx = -clamped_bary_deriv * 0.5;  // w0 decreases as we move right
    let dbary1_dx = clamped_bary_deriv * 0.866;  // w1 increases diagonally
    let dbary2_dx = clamped_bary_deriv * 0.366;  // w2 increases slightly

    let dbary0_dy = -clamped_bary_deriv * 0.866; // w0 decreases as we move up
    let dbary1_dy = clamped_bary_deriv * 0.5;    // w1 increases as we move up
    let dbary2_dy = clamped_bary_deriv;          // w2 increases more as we move up

    // Compute UV derivatives using chain rule: dUV/dx = sum(dwi/dx * uvi)
    let dudx = dbary0_dx * uv0.x + dbary1_dx * uv1.x + dbary2_dx * uv2.x;
    let dudy = dbary0_dy * uv0.x + dbary1_dy * uv1.x + dbary2_dy * uv2.x;
    let dvdx = dbary0_dx * uv0.y + dbary1_dx * uv1.y + dbary2_dx * uv2.y;
    let dvdy = dbary0_dy * uv0.y + dbary1_dy * uv1.y + dbary2_dy * uv2.y;

    return UvDerivs(dudx, dudy, dvdx, dvdy);
}

// LOD must be computed in *atlas* UV space, because the atlas' mip pyramid
// and sampleLevel() operate in that domain.
fn compute_texture_lod_atlas_space(
    tex: TextureInfo,
    tri: vec3<u32>,
    barycentric: vec3<f32>,
    attribute_data_offset: u32,
    vertex_stride: u32,
    cache: MipCache,
    screen_dims: vec2<f32>
) -> f32 {
    let uv_center_local = texture_uv(attribute_data_offset, tri, barycentric, tex, vertex_stride);

    // Try to compute accurate screen-space derivatives first
    var d: UvDerivs = uv_derivs_local(tex, tri, attribute_data_offset, vertex_stride, cache);

    // Check if we got valid derivatives
    let has_valid_derivs = (d.dudx != 0.0 || d.dudy != 0.0 || d.dvdx != 0.0 || d.dvdy != 0.0);

    let atlas = get_atlas_info(tex.atlas_index);

    // If we don't have valid derivatives, return a conservative mid-level LOD
    // This avoids harsh discontinuities from jumping to mip 0
    if (!has_valid_derivs) {
        // Use a conservative LOD that's reasonable for most cases
        // Mip level 1-2 is usually a good middle ground
        let conservative_lod = 1.5 + MIPMAP_GLOBAL_LOD_BIAS;
        let max_lod = max(atlas.levels_f - 1.0, 0.0);
        return clamp(conservative_lod, 0.0, max_lod);
    }

    if (!atlas.valid) {
        let uv0 = _texture_uv_per_vertex(attribute_data_offset, tex.attribute_uv_set_index, tri.x, vertex_stride);
        let uv1 = _texture_uv_per_vertex(attribute_data_offset, tex.attribute_uv_set_index, tri.y, vertex_stride);
        let uv2 = _texture_uv_per_vertex(attribute_data_offset, tex.attribute_uv_set_index, tri.z, vertex_stride);
        let du = max(max(abs(uv0.x - uv1.x), abs(uv1.x - uv2.x)), abs(uv2.x - uv0.x));
        let dv = max(max(abs(uv0.y - uv1.y), abs(uv1.y - uv2.y)), abs(uv2.y - uv0.y));
        let rho = max(du * f32(tex.size.x), dv * f32(tex.size.y));
        var lod_fb = log2(max(rho, 1e-6));
        lod_fb = clamp(lod_fb + MIPMAP_GLOBAL_LOD_BIAS, 0.0, 0.0);
        return atlas_clamp_cap(lod_fb, tex, uv_center_local);
    }

    // Convert local-UV derivatives → texture-space texels/pixel
    // d.dudx is change in local UV (0-1 space) per screen pixel
    // We want texels per screen pixel
    let dudx_texels = d.dudx * f32(tex.size.x);
    let dudy_texels = d.dudy * f32(tex.size.x);
    let dvdx_texels = d.dvdx * f32(tex.size.y);
    let dvdy_texels = d.dvdy * f32(tex.size.y);

    // Compute rho: maximum rate of change in texels per pixel
    // The issue: our analytical derivatives from triangle edges give us the rate
    // across the entire triangle, but we need it normalized properly.
    //
    // Standard approach: use the length of the gradient vector
    let rho_x = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);
    let rho_y = sqrt(dudy_texels * dudy_texels + dvdy_texels * dvdy_texels);

    // Take max for isotropic filtering (anisotropic would use both)
    let gradient = max(rho_x, rho_y);

    // CORRECTION: The derivatives we compute are per-pixel in screen space,
    // but the standard formula assumes a specific normalization.
    // Empirically, we're getting values ~5-6x too large.
    // This suggests we need to scale down by approximately sqrt(2) * 2 ≈ 2.8
    // Let's use 0.35 as a correction factor (1/2.8 ≈ 0.35)
    //
    // NOTE: If you see banding artifacts at certain zoom levels, try adjusting this value.
    // Higher values (e.g., 0.5) = sharper textures but more aliasing
    // Lower values (e.g., 0.25) = blurrier textures but smoother transitions
    let corrected_gradient = gradient * 0.35;

    var lod = log2(max(corrected_gradient, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS;

    let max_lod = max(atlas.levels_f - 1.0, 0.0);
    lod = clamp(lod, 0.0, max_lod);

    lod = atlas_clamp_cap(lod, tex, uv_center_local);
    return lod;
}

fn atlas_clamp_cap(lod_in: f32, tex: TextureInfo, uv_center: vec2<f32>) -> f32 {
    var lod = lod_in;

    let clamp_u = (tex.address_mode_u == ADDRESS_MODE_CLAMP_TO_EDGE);
    let clamp_v = (tex.address_mode_v == ADDRESS_MODE_CLAMP_TO_EDGE);
    let oob_u = clamp_u && (uv_center.x < -MIPMAP_CLAMP_EPSILON || uv_center.x > 1.0 + MIPMAP_CLAMP_EPSILON);
    let oob_v = clamp_v && (uv_center.y < -MIPMAP_CLAMP_EPSILON || uv_center.y > 1.0 + MIPMAP_CLAMP_EPSILON);

    if (oob_u || oob_v) {
        let max_clamp_lod = log2(max(MIPMAP_ATLAS_PADDING - 1.0, 1.0));
        lod = min(lod, max_clamp_lod);
    }
    return lod;
}

fn uv_derivatives_at(
    plane_u_over_w: PlaneCoefficients,
    plane_v_over_w: PlaneCoefficients,
    plane_inv_w:   PlaneCoefficients,
    pixel_xy: vec2<f32>
) -> UvDerivs {
    let A = eval_plane(plane_u_over_w, pixel_xy); // (u/w)
    let C = eval_plane(plane_v_over_w, pixel_xy); // (v/w)
    let B = eval_plane(plane_inv_w,    pixel_xy); // (1/w)

    let safeB = max(abs(B), 1e-6);
    let denom = safeB * safeB;

    let dudx = (plane_u_over_w.dx * B - A * plane_inv_w.dx) / denom;
    let dudy = (plane_u_over_w.dy * B - A * plane_inv_w.dy) / denom;
    let dvdx = (plane_v_over_w.dx * B - C * plane_inv_w.dx) / denom;
    let dvdy = (plane_v_over_w.dy * B - C * plane_inv_w.dy) / denom;

    return UvDerivs(dudx, dudy, dvdx, dvdy);
}

fn fit_plane(
    p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>,
    v0: f32, v1: f32, v2: f32
) -> PlaneCoefficients {
    let det = p0.x*(p1.y - p2.y) + p1.x*(p2.y - p0.y) + p2.x*(p0.y - p1.y);
    if (abs(det) < MIPMAP_MIN_DET) {
        return PlaneCoefficients(0.0, 0.0, 0.0, false);
    }

    let inv_det = 1.0 / det;
    let base = (v0*(p1.x*p2.y - p2.x*p1.y) + v1*(p2.x*p0.y - p0.x*p2.y) + v2*(p0.x*p1.y - p1.x*p0.y)) * inv_det;
    let dx   = (v0*(p1.y - p2.y) + v1*(p2.y - p0.y) + v2*(p0.y - p1.y)) * inv_det;
    let dy   = (v0*(p2.x - p1.x) + v1*(p0.x - p2.x) + v2*(p1.x - p0.x)) * inv_det;

    return PlaneCoefficients(base, dx, dy, true);
}

fn eval_plane(plane: PlaneCoefficients, point: vec2<f32>) -> f32 {
    return plane.base + plane.dx * point.x + plane.dy * point.y;
}

// Get atlas dims + mip count for a given atlas index.
// Extend this switch when you add more atlas bindings.
fn get_atlas_info(atlas_index: u32) -> AtlasInfo {
    switch (atlas_index) {
        {% for i in 0..texture_atlas_len %}
        case {{ i }}u: {
            let dims = vec2<f32>(textureDimensions(atlas_tex_{{ i }}, 0u));
            let lvls = f32(textureNumLevels(atlas_tex_{{ i }}));
            return AtlasInfo(dims, lvls, true);
        }
        {% endfor %}
        default: {
            return AtlasInfo(vec2<f32>(0.0), 0.0, false);
        }
    }
}
