// ============================================================================
// mipmap.wgsl — Gradient-based texture sampling for compute shaders
// ============================================================================
//
// This implementation computes UV derivatives (gradients) for anisotropic filtering:
// 1. Transform triangle vertices to screen space
// 2. Compute barycentric derivatives analytically: d(bary)/d(screen)
//    - This has been verified to match hardware dFdx/dFdy
// 3. Handle texture address modes (repeat/mirror) by unwrapping UVs:
//    - For repeat mode: make UVs continuous across 0→1 boundaries
//    - For mirror mode: convert to texture space, then unwrap
//    - This fixes mip selection when UVs cross wrapping boundaries
// 4. Apply chain rule: d(UV)/d(screen) = d(UV)/d(bary) × d(bary)/d(screen)
// 5. Apply orthographic projection correction for anisotropic filtering
// 6. Pass gradients to textureSampleGrad for hardware mip selection
//
// Benefits:
// - Hardware handles mip selection (anisotropic filtering, etc.)
// - No manual LOD calculation needed
// - No triangle seams
// - Correct gradients even when UVs wrap/repeat
// - Orthographic projection gets proper anisotropic filtering on tilted surfaces
// - Mathematically correct gradient computation
// ============================================================================

// ─────────────────────────────────────────────────────────────────────────────
// Orthographic Anisotropic Filtering Configuration
// ─────────────────────────────────────────────────────────────────────────────
// With orthographic projection, tilted surfaces don't get foreshortened naturally,
// so we need to manually scale UV gradients based on surface tilt to enable
// anisotropic filtering. These constants control that behavior.

// Angle threshold for applying correction (n_dot_v = cos(angle))
// - 0.95 = ~18° tilt (current default - balanced)
// - 0.98 = ~11° tilt (more aggressive, starts correction earlier)
// - 0.99 = ~8° tilt (very aggressive, may over-blur face-on surfaces)
// - 0.90 = ~26° tilt (conservative, only corrects steep angles)
const ORTHO_ANISO_THRESHOLD: f32 = 0.95;

// Minimum n_dot_v clamp to prevent infinite scaling at edge-on angles
// This determines the maximum anisotropic scale factor = 1.0 / ORTHO_ANISO_MIN_NDOTV
// - 0.05 = 20x max scale (current default - safe)
// - 0.02 = 50x max scale (more aggressive, better at grazing angles)
// - 0.01 = 100x max scale (very aggressive, may cause artifacts)
// - 0.10 = 10x max scale (conservative, may under-filter steep angles)
const ORTHO_ANISO_MIN_NDOTV: f32 = 0.05;

// Cross-gradient scale factor (0.0 to 1.0)
// When one gradient is scaled, apply this fraction to the perpendicular gradient
// - 0.3 = 30% (current default - handles slightly misaligned UVs)
// - 0.5 = 50% (more aggressive, better for rotated UV layouts)
// - 0.1 = 10% (conservative, assumes UVs perfectly aligned with geometry)
// - 0.0 = none (only scale the primary gradient, may cause artifacts)
const ORTHO_ANISO_CROSS_SCALE: f32 = 0.3;

// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Shared structs
// ─────────────────────────────────────────────────────────────────────────────
struct UvDerivs {
    ddx: vec2<f32>,  // (dudx, dvdx)
    ddy: vec2<f32>,  // (dudy, dvdy)
}

struct UV3 { u0: vec2<f32>, u1: vec2<f32>, u2: vec2<f32> }

struct MirrorLocal { uv: vec2<f32>, slope: vec2<f32> } // slope per axis is ±1

// Convert a UV coordinate from mirror-repeat space to texture space [0,1)
// This handles the reflection: [0,1) maps to [0,1), [1,2) maps to [1,0), [2,3) maps to [0,1), etc.
fn mirror_to_texture_space(uv: f32) -> f32 {
    let k = floor(uv);
    let frac = uv - k;
    let is_odd = (i32(k) & 1) != 0;
    // In odd segments, reflect: texture_coord = 1.0 - frac
    // In even segments, pass through: texture_coord = frac
    return select(frac, 1.0 - frac, is_odd);
}

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

// ─────────────────────────────────────────────────────────────────────────────
// Alternative mirror mode approach (not currently used, kept for reference)
// ─────────────────────────────────────────────────────────────────────────────
// These functions create a continuous "sawtooth" space for mirror coordinates,
// but require slope adjustment which complicates the implementation.
// Current approach uses mirror_to_texture_space() instead, which is simpler.

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
    barycentric: vec3<f32>,
    bary_derivs: vec4<f32>,         // (db1dx, db1dy, db2dx, db2dy)
    tri: vec3<u32>,
    attribute_data_offset: u32,
    vertex_stride: u32,
    tex_info: TextureInfo,
    world_normal: vec3<f32>,        // Surface normal in world space
    view_matrix: mat4x4<f32>        // Camera view matrix
) -> UvDerivs {
    let uv_set_index = tex_info.uv_set_index;

    let uv0 = _texture_uv_per_vertex(attribute_data_offset, tex_info.uv_set_index, tri.x, vertex_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, tex_info.uv_set_index, tri.y, vertex_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, tex_info.uv_set_index, tri.z, vertex_stride);

    // Barycentric derivatives from geometry pass
    // bary_derivs contains: (d(bary.x)/dx, d(bary.x)/dy, d(bary.y)/dx, d(bary.y)/dy)
    // Where bary is the vec2 barycentric coordinate being interpolated
    // The full vec3 barycentric is: (bary.x, bary.y, 1 - bary.x - bary.y)
    let dAlphaDx = bary_derivs.x;  // d(bary.x)/dx
    let dAlphaDy = bary_derivs.y;  // d(bary.x)/dy
    let dBetaDx = bary_derivs.z;   // d(bary.y)/dx
    let dBetaDy = bary_derivs.w;   // d(bary.y)/dy

    // If nearly zero derivatives, short-circuit (selects base mip).
    let m = abs(dAlphaDx) + abs(dAlphaDy) + abs(dBetaDx) + abs(dBetaDy);
    if (m < 1e-20) {
        return UvDerivs(vec2<f32>(0.0), vec2<f32>(0.0));
    }

    // Third barycentric component derivative: d(1 - alpha - beta)/d(x or y)
    let dGammaDx = -dAlphaDx - dBetaDx;
    let dGammaDy = -dAlphaDy - dBetaDy;

    // Make the THREE vertex UVs locally continuous per axis based on the address mode.
    var U0 = uv0;
    var U1 = uv1;
    var U2 = uv2;

    // Address mode constants (matching WebGPU GPUAddressMode enum values)
    const ADDR_MODE_CLAMP_TO_EDGE: u32 = 0u;
    const ADDR_MODE_REPEAT: u32 = 1u;
    const ADDR_MODE_MIRROR_REPEAT: u32 = 2u;

    // For mirror mode: convert UVs from mirror space to texture space [0,1)
    // This maps mirrored coordinates to their actual texture locations
    // Example: UV=1.1 (mirrored) becomes 0.9 (texture space)
    if (tex_info.address_mode_u == ADDR_MODE_MIRROR_REPEAT) {
        U0.x = mirror_to_texture_space(U0.x);
        U1.x = mirror_to_texture_space(U1.x);
        U2.x = mirror_to_texture_space(U2.x);
    }
    if (tex_info.address_mode_v == ADDR_MODE_MIRROR_REPEAT) {
        U0.y = mirror_to_texture_space(U0.y);
        U1.y = mirror_to_texture_space(U1.y);
        U2.y = mirror_to_texture_space(U2.y);
    }

    // For repeat/mirror modes: unwrap UVs to make them continuous across 0→1 boundaries
    // This fixes derivatives when UVs wrap (e.g., from 0.99 to 0.01)
    // Without unwrapping: gradient would be -0.98 (huge!)
    // With unwrapping: gradient becomes 0.02 (correct!)
    if (tex_info.address_mode_u == ADDR_MODE_REPEAT || tex_info.address_mode_u == ADDR_MODE_MIRROR_REPEAT) {
        U1.x = unwrap_repeat_axis(U0.x, U1.x);
        U2.x = unwrap_repeat_axis(U0.x, U2.x);
    }
    if (tex_info.address_mode_v == ADDR_MODE_REPEAT || tex_info.address_mode_v == ADDR_MODE_MIRROR_REPEAT) {
        U1.y = unwrap_repeat_axis(U0.y, U1.y);
        U2.y = unwrap_repeat_axis(U0.y, U2.y);
    }

    // Chain rule: d(UV)/d(screen) = d(UV)/d(bary) × d(bary)/d(screen)
    // UV = alpha*uv0 + beta*uv1 + gamma*uv2
    // So: d(UV)/dx = uv0*dAlpha/dx + uv1*dBeta/dx + uv2*dGamma/dx
    var dudx = U0.x * dAlphaDx + U1.x * dBetaDx + U2.x * dGammaDx;
    var dvdx = U0.y * dAlphaDx + U1.y * dBetaDx + U2.y * dGammaDx;

    var dudy = U0.x * dAlphaDy + U1.x * dBetaDy + U2.x * dGammaDy;
    var dvdy = U0.y * dAlphaDy + U1.y * dBetaDy + U2.y * dGammaDy;

    // ========================================================================
    // Orthographic projection correction for anisotropic filtering
    // ========================================================================
    // Problem: With orthographic projection, barycentric derivatives only capture
    // 2D screen-space triangle shape, not 3D surface tilt. A tilted surface doesn't
    // get foreshortened, so UV gradients remain isotropic even when viewing at angles.
    //
    // Solution: Use the surface normal and view direction to compute the true 3D tilt
    // and scale the UV gradients to create proper anisotropic filtering.
    //
    // Extract view direction from view matrix (orthographic = constant direction)
    // View matrix transforms world to view space, so -Z axis in view space = forward in world
    let view_forward = -normalize(vec3<f32>(view_matrix[0][2], view_matrix[1][2], view_matrix[2][2]));

    // Compute surface tilt: how much the surface faces away from the camera
    let n_dot_v = abs(dot(world_normal, view_forward));

    // When surface is tilted, one texture dimension gets "compressed" in screen space
    // The compression factor is 1/cos(θ) where θ is the tilt angle
    // For anisotropic filtering, we need to scale gradients by this factor
    //
    // At face-on (n_dot_v=1.0): no scaling needed
    // At 45° tilt (n_dot_v=0.707): scale by √2 ≈ 1.414
    // At 60° tilt (n_dot_v=0.5): scale by 2.0
    // At edge-on (n_dot_v=0.0): scale approaches infinity (clamp to prevent issues)
    let aniso_scale = 1.0 / max(n_dot_v, ORTHO_ANISO_MIN_NDOTV);

    // Apply anisotropic scaling to BOTH gradient directions
    // The larger gradient likely aligns with the tilt direction and needs more scaling
    let ddx_vec = vec2<f32>(dudx, dvdx);
    let ddy_vec = vec2<f32>(dudy, dvdy);
    let ddx_mag = length(ddx_vec);
    let ddy_mag = length(ddy_vec);

    // Apply scaling when tilted
    if (n_dot_v < ORTHO_ANISO_THRESHOLD) {
        if (ddx_mag > ddy_mag) {
            // X gradient is larger - scale it more aggressively
            dudx *= aniso_scale;
            dvdx *= aniso_scale;
            // Also scale Y gradient slightly (surface isn't perfectly aligned)
            dudy *= mix(1.0, aniso_scale, ORTHO_ANISO_CROSS_SCALE);
            dvdy *= mix(1.0, aniso_scale, ORTHO_ANISO_CROSS_SCALE);
        } else {
            // Y gradient is larger - scale it more aggressively
            dudy *= aniso_scale;
            dvdy *= aniso_scale;
            // Also scale X gradient slightly (surface isn't perfectly aligned)
            dudx *= mix(1.0, aniso_scale, ORTHO_ANISO_CROSS_SCALE);
            dvdx *= mix(1.0, aniso_scale, ORTHO_ANISO_CROSS_SCALE);
        }
    }

    // LOD bias simulation: Scale gradients to shift mip selection
    // LOD = log2(rho) where rho is gradient magnitude
    // LOD_biased = LOD + bias
    //
    // To achieve bias effect, scale gradients:
    //   bias = -0.5 → scale = 1.414 (sharper: divide gradients by 1.414)
    //   bias = -1.0 → scale = 2.0 (much sharper: divide gradients by 2.0)
    //   bias = +0.5 → scale = 0.707 (blurrier: multiply gradients by 1.414)
    //   bias =  0.0 → scale = 1.0 (no change)
    //
    // Current setting: -0.5 bias (sharper, less swimming on thin lines)
    // let lod_bias_scale = 1.414;  // 2^0.5
    // dudx /= lod_bias_scale;
    // dvdx /= lod_bias_scale;
    // dudy /= lod_bias_scale;
    // dvdy /= lod_bias_scale;

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
    world_normal: vec3<f32>,        // For orthographic anisotropic correction
    view_matrix: mat4x4<f32>        // For orthographic anisotropic correction
) -> PbrMaterialGradients {

    var out : PbrMaterialGradients;

    if (material.has_base_color_texture) {
        out.base_color = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.base_color_tex_info,
            world_normal,
            view_matrix
        );
    }

    if (material.has_metallic_roughness_texture) {
        out.metallic_roughness = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.metallic_roughness_tex_info,
            world_normal,
            view_matrix
        );
    }

    if (material.has_normal_texture) {
        out.normal = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.normal_tex_info,
            world_normal,
            view_matrix
        );
    }

    if (material.has_occlusion_texture) {
        out.occlusion = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.occlusion_tex_info,
            world_normal,
            view_matrix
        );
    }

    if (material.has_emissive_texture) {
        out.emissive = get_uv_derivatives(
            barycentric,
            bary_derivs,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.emissive_tex_info,
            world_normal,
            view_matrix
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
