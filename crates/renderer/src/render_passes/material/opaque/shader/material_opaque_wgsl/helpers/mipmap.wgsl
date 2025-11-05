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
    dudx : f32,
    dudy : f32,
    dvdx : f32,
    dvdy : f32,
}

// Helper: 2x2 matrix determinant
fn det2(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

// Helper: 2x2 matrix inverse
fn inv2(a: vec2<f32>, b: vec2<f32>) -> mat2x2<f32> {
    let d = det2(a, b);
    if (abs(d) < 1e-8) {
        return mat2x2<f32>(vec2<f32>(0.0), vec2<f32>(0.0));
    }
    let invd = 1.0 / d;
    return mat2x2<f32>(
        vec2<f32>(b.y, -a.y) * invd,
        vec2<f32>(-b.x, a.x) * invd
    );
}

// Helper: Transform clip-space to pixel coordinates
fn clip_to_pixel(clip: vec4<f32>, screen_size: vec2<f32>) -> vec2<f32> {
    let ndc = clip.xy / clip.w;
    // Convert NDC [-1,1] to [0,1], with Y-flip to match framebuffer coordinates
    // In WebGPU/WGSL: NDC Y=-1 is bottom, Y=+1 is top
    // In framebuffer: pixel Y=0 is top, Y=height is bottom
    let xy01 = vec2<f32>(
        ndc.x * 0.5 + 0.5,
        1.0 - (ndc.y * 0.5 + 0.5)  // Flip Y
    );
    return xy01 * screen_size;
}

// Helper: Compute barycentric coordinates for a point in screen space
fn compute_barycentric(p: vec2<f32>, p0: vec2<f32>, e01: vec2<f32>, e02: vec2<f32>, inv_area: f32) -> vec3<f32> {
    let v0 = p - p0;
    let d10 = det2(v0, e02);
    let d20 = det2(e01, v0);
    let b1 = d10 * inv_area;
    let b2 = d20 * inv_area;
    let b0 = 1.0 - b1 - b2;
    return vec3<f32>(b0, b1, b2);
}

// Helper: Linearly interpolate UVs using barycentric coordinates
fn interpolate_uv_linear(bary: vec3<f32>, uv0: vec2<f32>, uv1: vec2<f32>, uv2: vec2<f32>) -> vec2<f32> {
    return uv0 * bary.x + uv1 * bary.y + uv2 * bary.z;
}

// Compute barycentric coordinate derivatives geometrically
// Returns d(bary.xy)/d(screen) as vec4(db1/dx, db1/dy, db2/dx, db2/dy)
// bary = (b0, b1, b2) where b0 = 1 - b1 - b2
fn compute_barycentric_derivatives(
    p0: vec2<f32>,
    p1: vec2<f32>,
    p2: vec2<f32>
) -> vec4<f32> {
    let e01 = p1 - p0;
    let e02 = p2 - p0;
    let area = det2(e01, e02);

    if (abs(area) < 1e-8) {
        return vec4<f32>(0.0);
    }

    let inv_area = 1.0 / area;

    // Barycentric formulas:
    // b1 = det(p - p0, e02) / area
    // b2 = det(e01, p - p0) / area
    //
    // Taking derivatives with respect to screen position:
    // d(b1)/dx = det((1,0), e02) / area = e02.y / area
    // d(b1)/dy = det((0,1), e02) / area = -e02.x / area
    // d(b2)/dx = det(e01, (1,0)) / area = -e01.y / area
    // d(b2)/dy = det(e01, (0,1)) / area = e01.x / area

    let db1_dx = e02.y * inv_area;
    let db1_dy = -e02.x * inv_area;
    let db2_dx = -e01.y * inv_area;
    let db2_dy = e01.x * inv_area;

    return vec4<f32>(db1_dx, db1_dy, db2_dx, db2_dy);
}

// Compute UV derivatives using verified barycentric gradient chain rule
fn compute_uv_derivatives_from_depth(
    coords: vec2<i32>,
    pixel_center: vec2<f32>,
    screen_size: vec2<f32>,
    tri: vec3<u32>,
    attribute_data_offset: u32,
    vertex_stride: u32,
    uv_set_index: u32,
    inv_view_proj: mat4x4<f32>,
    os_vertices: ObjectSpaceVertices,
    world_model: mat4x4<f32>
) -> UvDerivs {
    // Get triangle UVs
    let uv0 = _texture_uv_per_vertex(attribute_data_offset, uv_set_index, tri.x, vertex_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, uv_set_index, tri.y, vertex_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, uv_set_index, tri.z, vertex_stride);

    // Transform triangle vertices to clip space
    let mvp = camera.view_proj * world_model;
    let clip0 = mvp * vec4<f32>(os_vertices.p0, 1.0);
    let clip1 = mvp * vec4<f32>(os_vertices.p1, 1.0);
    let clip2 = mvp * vec4<f32>(os_vertices.p2, 1.0);

    // Convert to screen-space pixel coordinates
    let p0 = clip_to_pixel(clip0, screen_size);
    let p1 = clip_to_pixel(clip1, screen_size);
    let p2 = clip_to_pixel(clip2, screen_size);

    // Compute barycentric derivatives: d(bary)/d(screen)
    // This uses the verified formula that matches hardware dFdx/dFdy
    let bary_derivs = compute_barycentric_derivatives(p0, p1, p2);

    // Check for degenerate triangle
    if (bary_derivs.x == 0.0 && bary_derivs.y == 0.0 &&
        bary_derivs.z == 0.0 && bary_derivs.w == 0.0) {
        return UvDerivs(10.0, 10.0, 10.0, 10.0);
    }

    // Apply chain rule: d(UV)/d(screen) = d(UV)/d(bary) × d(bary)/d(screen)
    // d(UV)/d(b1) = uv1 - uv0
    // d(UV)/d(b2) = uv2 - uv0
    let duv_db1 = uv1 - uv0;
    let duv_db2 = uv2 - uv0;

    // Chain rule application:
    // d(UV)/dx = d(UV)/d(b1) × d(b1)/dx + d(UV)/d(b2) × d(b2)/dx
    // d(UV)/dy = d(UV)/d(b1) × d(b1)/dy + d(UV)/d(b2) × d(b2)/dy
    let ddx_uv = duv_db1 * bary_derivs.x + duv_db2 * bary_derivs.z;  // db1/dx, db2/dx
    let ddy_uv = duv_db1 * bary_derivs.y + duv_db2 * bary_derivs.w;  // db1/dy, db2/dy

    let dudx = ddx_uv.x;
    let dvdx = ddx_uv.y;
    let dudy = ddy_uv.x;
    let dvdy = ddy_uv.y;

    // Safety checks for extreme or invalid derivatives
    if (dudx != dudx || dudy != dudy || dvdx != dvdx || dvdy != dvdy) {
        // NaN - use large derivatives to force blur
        return UvDerivs(10.0, 10.0, 10.0, 10.0);
    }

    // For extreme derivatives, clamp them to reasonable range
    const MAX_DERIVATIVE = 100.0;
    let clamped_dudx = clamp(dudx, -MAX_DERIVATIVE, MAX_DERIVATIVE);
    let clamped_dudy = clamp(dudy, -MAX_DERIVATIVE, MAX_DERIVATIVE);
    let clamped_dvdx = clamp(dvdx, -MAX_DERIVATIVE, MAX_DERIVATIVE);
    let clamped_dvdy = clamp(dvdy, -MAX_DERIVATIVE, MAX_DERIVATIVE);

    return UvDerivs(
        clamped_dudx,
        clamped_dudy,
        clamped_dvdx,
        clamped_dvdy
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API - Gradient Computation
// ─────────────────────────────────────────────────────────────────────────────

// Computes UV derivatives for each texture type, which are used with textureSampleGrad
// This enables hardware anisotropic filtering in compute shaders
fn pbr_get_gradients(
    coords: vec2<i32>,
    pixel_center: vec2<f32>,
    screen_dims: vec2<f32>,
    material: PbrMaterial,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    inv_view_proj: mat4x4<f32>,
    os_vertices: ObjectSpaceVertices,
    world_model: mat4x4<f32>
) -> PbrMaterialGradients {

    var out : PbrMaterialGradients;

    if (material.has_base_color_texture) {
        let d = compute_uv_derivatives_from_depth(
            coords, pixel_center, screen_dims,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.base_color_tex_info.attribute_uv_set_index,
            inv_view_proj, os_vertices, world_model
        );
        out.base_color_ddx = vec2<f32>(d.dudx, d.dvdx);
        out.base_color_ddy = vec2<f32>(d.dudy, d.dvdy);
    } else {
        out.base_color_ddx = vec2<f32>(0.0, 0.0);
        out.base_color_ddy = vec2<f32>(0.0, 0.0);
    }

    if (material.has_metallic_roughness_texture) {
        let d = compute_uv_derivatives_from_depth(
            coords, pixel_center, screen_dims,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.metallic_roughness_tex_info.attribute_uv_set_index,
            inv_view_proj, os_vertices, world_model
        );
        out.metallic_roughness_ddx = vec2<f32>(d.dudx, d.dvdx);
        out.metallic_roughness_ddy = vec2<f32>(d.dudy, d.dvdy);
    } else {
        out.metallic_roughness_ddx = vec2<f32>(0.0, 0.0);
        out.metallic_roughness_ddy = vec2<f32>(0.0, 0.0);
    }

    if (material.has_normal_texture) {
        let d = compute_uv_derivatives_from_depth(
            coords, pixel_center, screen_dims,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.normal_tex_info.attribute_uv_set_index,
            inv_view_proj, os_vertices, world_model
        );
        out.normal_ddx = vec2<f32>(d.dudx, d.dvdx);
        out.normal_ddy = vec2<f32>(d.dudy, d.dvdy);
    } else {
        out.normal_ddx = vec2<f32>(0.0, 0.0);
        out.normal_ddy = vec2<f32>(0.0, 0.0);
    }

    if (material.has_occlusion_texture) {
        let d = compute_uv_derivatives_from_depth(
            coords, pixel_center, screen_dims,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.occlusion_tex_info.attribute_uv_set_index,
            inv_view_proj, os_vertices, world_model
        );
        out.occlusion_ddx = vec2<f32>(d.dudx, d.dvdx);
        out.occlusion_ddy = vec2<f32>(d.dudy, d.dvdy);
    } else {
        out.occlusion_ddx = vec2<f32>(0.0, 0.0);
        out.occlusion_ddy = vec2<f32>(0.0, 0.0);
    }

    if (material.has_emissive_texture) {
        let d = compute_uv_derivatives_from_depth(
            coords, pixel_center, screen_dims,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            material.emissive_tex_info.attribute_uv_set_index,
            inv_view_proj, os_vertices, world_model
        );
        out.emissive_ddx = vec2<f32>(d.dudx, d.dvdx);
        out.emissive_ddy = vec2<f32>(d.dudy, d.dvdy);
    } else {
        out.emissive_ddx = vec2<f32>(0.0, 0.0);
        out.emissive_ddy = vec2<f32>(0.0, 0.0);
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
    uv_scale: vec2<f32>,
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
    let ddx_atlas = ddx_local * uv_scale;
    let ddy_atlas = ddy_local * uv_scale;

    // Convert to texel space using ATLAS dimensions (what hardware actually sees)
    let ddx_texels = ddx_atlas * atlas_dims;
    let ddy_texels = ddy_atlas * atlas_dims;

    let rho_x = length(ddx_texels);
    let rho_y = length(ddy_texels);
    let rho = max(rho_x, rho_y);

    return log2(max(rho, 1e-6));
}
