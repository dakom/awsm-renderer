// ============================================================================
// mipmap.wgsl — Gradient-based texture sampling for compute shaders
// ============================================================================
//
// This implementation computes UV derivatives (gradients) for anisotropic filtering:
// 1. Transform triangle vertices to screen space
// 2. Compute barycentric derivatives analytically: d(bary)/d(screen)
//    - This has been verified to match hardware dFdx/dFdy
// 3. Apply chain rule: d(UV)/d(screen) = d(UV)/d(bary) × d(bary)/d(screen)
// 4. Scale by atlas uv_scale transform
// 5. Pass gradients to textureSampleGrad for hardware mip selection
//
// Benefits:
// - Hardware handles mip selection (anisotropic filtering, etc.)
// - No manual LOD calculation needed
// - No triangle seams
// - Mathematically correct gradient computation
//
// Note: There appears to be a systematic 4x gradient magnitude discrepancy between
// this geometric calculation and what produces optimal mip selection. The root cause
// is under investigation - see DEBUG_MIPMAPS.md for current status and next steps.
// ============================================================================

// ─────────────────────────────────────────────────────────────────────────────
// Shared structs
// ─────────────────────────────────────────────────────────────────────────────
struct UvDerivs {
    ddx: vec2<f32>,  // (dudx, dvdx)
    ddy: vec2<f32>,  // (dudy, dvdy)
}

struct UV3 { u0: vec2<f32>, u1: vec2<f32>, u2: vec2<f32> }

struct MirrorLocal { uv: vec2<f32>, slope: vec2<f32> } // slope per axis is ±1

// Unwrap aX relative to a0 for repeat textures
// This makes vertices continuous when they cross the 0→1 boundary,
// but preserves large spans (e.g., texture repeating multiple times)
fn unwrap_repeat_axis(a0: f32, aX: f32) -> f32 {
    let d = aX - a0;

    // Compute what the distance would be with ±1 shifts
    // Choose whichever gives the smallest absolute distance
    let d_minus = d - 1.0;
    let d_plus = d + 1.0;

    // But only apply a shift if it makes a "significant" improvement (> 0.5 reduction)
    // This prevents collapsing large spans while still unwrapping boundaries
    if (abs(d_plus) < abs(d) - 0.5) {
        return aX + 1.0;
    } else if (abs(d_minus) < abs(d) - 0.5) {
        return aX - 1.0;
    } else {
        return aX;
    }
}


fn unwrap_repeat(u0: vec2<f32>, u1: vec2<f32>, u2: vec2<f32>) -> UV3 {
    return UV3(
        u0,
        vec2<f32>(unwrap_repeat_axis(u0.x, u1.x), unwrap_repeat_axis(u0.y, u1.y)),
        vec2<f32>(unwrap_repeat_axis(u0.x, u2.x), unwrap_repeat_axis(u0.y, u2.y))
    );
}
fn mirror_linearize_axis(a: f32) -> vec2<f32> {
    // Returns (u_lin, slope)
    let k = floor(a);
    let frac = a - k; // [0,1)
    let is_odd = (i32(k) & 1) != 0;
    // Map to a monotonic coordinate (continuous in "mirror space"):
    let u_lin = select(frac, 1.0 - frac, is_odd) + k;
    let s = select(1.0, -1.0, is_odd);
    return vec2<f32>(u_lin, s);
}

fn mirror_linearize(uv: vec2<f32>) -> MirrorLocal {
    let x = mirror_linearize_axis(uv.x);
    let y = mirror_linearize_axis(uv.y);
    return MirrorLocal(
        vec2<f32>(x.x, y.x),
        vec2<f32>(x.y, y.y)
    );
}

fn get_uv_derivatives(
    barycentric: vec3<f32>,         // (db1dx, db1dy, db2dx, db2dy)
    bary_derivs: vec4<f32>,         // (db1dx, db1dy, db2dx, db2dy)
    tri: vec3<u32>,
    attribute_data_offset: u32,
    vertex_stride: u32,
    tex_info: TextureInfo
) -> UvDerivs {
    let uv_set_index = tex_info.attribute_uv_set_index;

    // Fetch per-vertex UVs (raw, as authored)
        let uv0 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, tri.x, vertex_stride);
        let uv1 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, tri.y, vertex_stride);
        let uv2 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, tri.z, vertex_stride);

        let db1dx = bary_derivs.x;
        let db1dy = bary_derivs.y;
        let db2dx = bary_derivs.z;
        let db2dy = bary_derivs.w;

        // If nearly zero derivatives, short-circuit (selects base mip).
        let m = abs(db1dx) + abs(db1dy) + abs(db2dx) + abs(db2dy);
        if (m < 1e-20) {
            return UvDerivs(vec2<f32>(0.0), vec2<f32>(0.0));
        }

        // Perspective barycentrics: b0 = 1 - b1 - b2
        let db0dx = -db1dx - db2dx;
        let db0dy = -db1dy - db2dy;

        // Make the THREE vertex UVs locally continuous per axis based on the address mode.
        var U0 = uv0;
        var U1 = uv1;
        var U2 = uv2;

        // TEMPORARY FIX: Skip unwrapping for REPEAT mode entirely
        // The unwrapping logic was collapsing large UV spans, causing incorrect mip selection
        // For MIRROR_REPEAT, we still need linearization + unwrapping
        // Handle U axis:
        switch (tex_info.address_mode_u) {
            case ADDRESS_MODE_REPEAT: {
                // Don't unwrap - use raw UVs
                // This preserves large spans but may have seams at boundaries
            }
            case ADDRESS_MODE_MIRROR_REPEAT: {
                let L0 = mirror_linearize(vec2<f32>(U0.x, 0.0));
                let L1 = mirror_linearize(vec2<f32>(U1.x, 0.0));
                let L2 = mirror_linearize(vec2<f32>(U2.x, 0.0));
                let R  = unwrap_repeat(vec2<f32>(L0.uv.x, 0.0), vec2<f32>(L1.uv.x, 0.0), vec2<f32>(L2.uv.x, 0.0));
                U0.x = R.u0.x; U1.x = R.u1.x; U2.x = R.u2.x;
            }
            default: { /* CLAMP: nothing */ }
        }

        // Handle V axis:
        switch (tex_info.address_mode_v) {
            case ADDRESS_MODE_REPEAT: {
                // Don't unwrap - use raw UVs
            }
            case ADDRESS_MODE_MIRROR_REPEAT: {
                let L0 = mirror_linearize(vec2<f32>(0.0, U0.y));
                let L1 = mirror_linearize(vec2<f32>(0.0, U1.y));
                let L2 = mirror_linearize(vec2<f32>(0.0, U2.y));
                let R  = unwrap_repeat(vec2<f32>(0.0, L0.uv.y), vec2<f32>(0.0, L1.uv.y), vec2<f32>(0.0, L2.uv.y));
                U0.y = R.u0.y; U1.y = R.u1.y; U2.y = R.u2.y;
            }
            default: { /* CLAMP: nothing */ }
        }

        // Chain rule with the unwrapped / linearized UVs
        var dudx = U0.x * db0dx + U1.x * db1dx + U2.x * db2dx;
        var dvdx = U0.y * db0dx + U1.y * db1dx + U2.y * db2dx;

        var dudy = U0.x * db0dy + U1.x * db1dy + U2.x * db2dy;
        var dvdy = U0.y * db0dy + U1.y * db1dy + U2.y * db2dy;

        // For MIRROR_REPEAT, apply local slope sign (+1/-1) at THIS PIXEL so grads reflect flips.
        if (tex_info.address_mode_u == ADDRESS_MODE_MIRROR_REPEAT ||
            tex_info.address_mode_v == ADDRESS_MODE_MIRROR_REPEAT) {

            // Interpolated raw UV at this pixel (no wrapping) just to decide parity:
            let uv_pix = barycentric.x * uv0 + barycentric.y * uv1 + barycentric.z * uv2;

            if (tex_info.address_mode_u == ADDRESS_MODE_MIRROR_REPEAT) {
                let sx = mirror_linearize_axis(uv_pix.x).y; // +1 or -1
                dudx *= sx; dudy *= sx;
            }
            if (tex_info.address_mode_v == ADDRESS_MODE_MIRROR_REPEAT) {
                let sy = mirror_linearize_axis(uv_pix.y).y; // +1 or -1
                dvdx *= sy; dvdy *= sy;
            }
        }

        // NaN/Inf guard (don’t clamp magnitudes)
        let ok = (dudx == dudx) && (dudy == dudy) && (dvdx == dvdx) && (dvdy == dvdy);
        if (!ok) {
            return UvDerivs(vec2<f32>(0.0), vec2<f32>(0.0));
        }

        return UvDerivs(vec2<f32>(dudx, dvdx), vec2<f32>(dudy, dvdy));
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API - Gradient Computation
// ─────────────────────────────────────────────────────────────────────────────

// Computes UV derivatives for each texture type, which are used with textureSampleGrad
// This enables hardware anisotropic filtering in compute shaders
fn pbr_get_gradients(
    barycentric: vec3<f32>,         // (b0, b1, b2)
    bary_derivs: vec4<f32>,         // (db1dx, db1dy, db2dx, db2dy)
    material: PbrMaterial,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
) -> PbrMaterialGradients {

    var out : PbrMaterialGradients;

    if (material.has_base_color_texture) {
        out.base_color = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.base_color_tex_info,
        );
    }

    if (material.has_metallic_roughness_texture) {
        out.metallic_roughness = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.metallic_roughness_tex_info,
        );
    }

    if (material.has_normal_texture) {
        out.normal = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.normal_tex_info,
        );
    }

    if (material.has_occlusion_texture) {
        out.occlusion = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.occlusion_tex_info,
        );
    }

    if (material.has_emissive_texture) {
        out.emissive = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.emissive_tex_info,
        );
    }

    return out;
}

// Debug helper: Calculate what mip level would be selected for a given texture
// This mimics the hardware mip selection algorithm
fn debug_calculate_mip_level(
    ddx: vec2<f32>,
    ddy: vec2<f32>,
    texture_size: vec2<u32>
) -> f32 {
    // Convert gradients from UV space [0,1] to texel space
    let ddx_texels = ddx * vec2<f32>(f32(texture_size.x), f32(texture_size.y));
    let ddy_texels = ddy * vec2<f32>(f32(texture_size.x), f32(texture_size.y));

    // Compute gradient magnitudes (texels per pixel)
    let rho_x = length(ddx_texels);
    let rho_y = length(ddy_texels);
    let rho = max(rho_x, rho_y);

    // Hardware mip selection: LOD = log2(rho)
    return log2(max(rho, 1e-6));
}

// Debug helper: Calculate actual atlas mip level (what hardware selects)
// Takes atlas dimensions to compute the real mip level
fn debug_calculate_atlas_mip_level(
    ddx_local: vec2<f32>,
    ddy_local: vec2<f32>,
    grad_scale: vec2<f32>,
    atlas_index: u32
) -> f32 {
    // Get atlas dimensions
    var atlas_dims = vec2<f32>(0.0);
    switch (atlas_index) {
        {% for i in 0..texture_atlas_len %}
        case {{ i }}u: {
            atlas_dims = vec2<f32>(textureDimensions(atlas_tex_{{ i }}, 0u));
        }
        {% endfor %}
        default: {}
    }

    // Convert from local UV space to atlas UV space
    let ddx_atlas = ddx_local * grad_scale;
    let ddy_atlas = ddy_local * grad_scale;

    // Convert to texel space using ATLAS dimensions (what hardware actually sees)
    let ddx_texels = ddx_atlas * atlas_dims;
    let ddy_texels = ddy_atlas * atlas_dims;

    let rho_x = length(ddx_texels);
    let rho_y = length(ddy_texels);
    let rho = max(rho_x, rho_y);

    return log2(max(rho, 1e-6));
}
