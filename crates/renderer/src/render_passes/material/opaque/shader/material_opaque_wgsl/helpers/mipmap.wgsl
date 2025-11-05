// ============================================================================
// mipmap.wgsl — Gradient-based texture sampling for compute shaders
// ============================================================================
//
// This implementation computes UV derivatives (gradients) for anisotropic filtering:
// 1. Transform triangle vertices to screen space
// 2. Compute screen-space Jacobian (dScreen/dBarycentric)
// 3. Invert to get dBarycentric/dScreen
// 4. Chain with dUV/dBarycentric to get dUV/dScreen (the gradients we need)
// 5. Pass gradients to textureSampleGrad for hardware mip selection
//
// Benefits:
// - Hardware handles mip selection (anisotropic filtering, etc.)
// - No manual LOD calculation needed
// - No triangle seams
// - Matches fragment shader quality
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

// Compute UV derivatives using screen-space Jacobian
// This is simpler and more robust than world-space reconstruction
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

    // Screen-space triangle edges
    let e01_screen = p1 - p0;
    let e02_screen = p2 - p0;

    // Check for degenerate triangle in screen space
    let screen_area = abs(det2(e01_screen, e02_screen));
    if (screen_area < 0.01) {
        // Triangle is too small in screen space (< 0.01 pixels² area)
        // Use LARGE derivatives to force highest mip level (maximum blur)
        // Large derivatives = many texels per pixel = need blur
        return UvDerivs(10.0, 10.0, 10.0, 10.0);
    }

    // Compute inverse of screen-space triangle matrix
    // Minv maps from screen-space displacement to barycentric coords
    let Minv = inv2(e01_screen, e02_screen);

    // Check if inversion failed (det was too small)
    if (Minv[0].x == 0.0 && Minv[0].y == 0.0 &&
        Minv[1].x == 0.0 && Minv[1].y == 0.0) {
        return UvDerivs(10.0, 10.0, 10.0, 10.0);
    }

    // UV space triangle edges
    let e01_uv = uv1 - uv0;
    let e02_uv = uv2 - uv0;

    // Compute UV Jacobian: J = [e01_uv, e02_uv] * Minv
    // This gives us d(uv)/d(screen) - the derivatives we need!
    // J[0] = d(uv)/dx (first column)
    // J[1] = d(uv)/dy (second column)
    let T = mat2x2<f32>(e01_uv, e02_uv);
    let J = T * Minv;

    let dudx = J[0].x;
    let dvdx = J[0].y;
    let dudy = J[1].x;
    let dvdy = J[1].y;

    // Safety checks for extreme or invalid derivatives
    if (dudx != dudx || dudy != dudy || dvdx != dvdx || dvdy != dvdy) {
        // NaN - use large derivatives to force blur
        return UvDerivs(10.0, 10.0, 10.0, 10.0);
    }

    // For extreme derivatives, clamp them to reasonable range
    // Don't reject them entirely - just limit the max LOD
    const MAX_DERIVATIVE = 100.0;
    let clamped_dudx = clamp(dudx, -MAX_DERIVATIVE, MAX_DERIVATIVE);
    let clamped_dudy = clamp(dudy, -MAX_DERIVATIVE, MAX_DERIVATIVE);
    let clamped_dvdx = clamp(dvdx, -MAX_DERIVATIVE, MAX_DERIVATIVE);
    let clamped_dvdy = clamp(dvdy, -MAX_DERIVATIVE, MAX_DERIVATIVE);

    return UvDerivs(clamped_dudx, clamped_dudy, clamped_dvdx, clamped_dvdy);
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
