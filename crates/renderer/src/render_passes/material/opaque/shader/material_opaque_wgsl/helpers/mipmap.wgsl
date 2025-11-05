// ============================================================================
// mipmap.wgsl — Per-pixel mip selection using depth buffer reconstruction
// ============================================================================
//
// This implementation computes texture LOD per-pixel by:
// 1. Reconstructing world positions from depth buffer (current pixel + neighbors)
// 2. Computing world-space derivatives (dWorld/dx, dWorld/dy)
// 3. Using barycentric interpolation to get UV derivatives (dUV/dx, dUV/dy)
// 4. Standard LOD formula: LOD = log2(max(length(dUV/dx), length(dUV/dy)) * texture_size)
//
// Benefits over per-triangle approach:
// - No triangle seams (smooth LOD across surfaces)
// - Works perfectly for repeating textures (no special handling needed)
// - No cache complexity
// - Matches fragment shader quality exactly
// ============================================================================

const MIPMAP_GLOBAL_LOD_BIAS : f32 = -0.5;
const MIPMAP_CLAMP_EPSILON   : f32 = 1e-4;
const MIPMAP_ATLAS_PADDING   : f32 = 8.0;

// ─────────────────────────────────────────────────────────────────────────────
// Shared structs
// ─────────────────────────────────────────────────────────────────────────────
struct UvDerivs {
    dudx : f32,
    dudy : f32,
    dvdx : f32,
    dvdy : f32,
}

// Legacy struct - kept for compatibility with zero-LOD paths
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
// World Position Reconstruction from Depth
// ─────────────────────────────────────────────────────────────────────────────

// Reconstruct world-space position from depth buffer
// Based on standard deferred rendering approach
fn reconstruct_world_position(
    pixel_coord: vec2<f32>,
    depth: f32,
    inv_view_proj: mat4x4<f32>,
    screen_size: vec2<f32>
) -> vec3<f32> {
    // Convert pixel coordinates to NDC [-1, 1]
    // Note: Y is inverted in screen space
    let ndc = vec2<f32>(
        (pixel_coord.x / screen_size.x) * 2.0 - 1.0,
        1.0 - (pixel_coord.y / screen_size.y) * 2.0
    );

    // Construct clip-space position (WebGPU uses depth range [0, 1])
    let clip_pos = vec4<f32>(ndc, depth, 1.0);

    // Transform to world space and apply perspective divide
    let world_pos = inv_view_proj * clip_pos;
    return world_pos.xyz / world_pos.w;
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
// Public API
// ─────────────────────────────────────────────────────────────────────────────

// NEW: Gradient-based API for anisotropic filtering
// Computes UV derivatives for each texture type, which are used with textureSampleGrad
// This approach:
// - Leverages hardware gradient computation (faster)
// - Enables anisotropic filtering automatically
// - More robust than manual LOD calculation
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

// LEGACY: LOD-based API (kept for compatibility)
fn pbr_get_mipmap_levels(
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
) -> PbrMaterialMipLevels {

    var out : PbrMaterialMipLevels;

    if (material.has_base_color_texture) {
        out.base_color = compute_texture_lod_from_depth(
            coords, pixel_center, screen_dims,
            material.base_color_tex_info,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            inv_view_proj, os_vertices, world_model
        );
    } else { out.base_color = 0.0; }

    if (material.has_metallic_roughness_texture) {
        out.metallic_roughness = compute_texture_lod_from_depth(
            coords, pixel_center, screen_dims,
            material.metallic_roughness_tex_info,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            inv_view_proj, os_vertices, world_model
        );
    } else { out.metallic_roughness = 0.0; }

    if (material.has_normal_texture) {
        out.normal = compute_texture_lod_from_depth(
            coords, pixel_center, screen_dims,
            material.normal_tex_info,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            inv_view_proj, os_vertices, world_model
        );
    } else { out.normal = 0.0; }

    if (material.has_occlusion_texture) {
        out.occlusion = compute_texture_lod_from_depth(
            coords, pixel_center, screen_dims,
            material.occlusion_tex_info,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            inv_view_proj, os_vertices, world_model
        );
    } else { out.occlusion = 0.0; }

    if (material.has_emissive_texture) {
        out.emissive = compute_texture_lod_from_depth(
            coords, pixel_center, screen_dims,
            material.emissive_tex_info,
            triangle_indices,
            attribute_data_offset, vertex_attribute_stride,
            inv_view_proj, os_vertices, world_model
        );
    } else { out.emissive = 0.0; }

    return out;
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

// Compute texture LOD using depth buffer reconstruction
// This is the new, correct approach for deferred rendering
fn compute_texture_lod_from_depth(
    coords: vec2<i32>,
    pixel_center: vec2<f32>,
    screen_size: vec2<f32>,
    tex: TextureInfo,
    tri: vec3<u32>,
    attribute_data_offset: u32,
    vertex_stride: u32,
    inv_view_proj: mat4x4<f32>,
    os_vertices: ObjectSpaceVertices,
    world_model: mat4x4<f32>
) -> f32 {
    // Compute UV derivatives using depth buffer reconstruction
    let d = compute_uv_derivatives_from_depth(
        coords, pixel_center, screen_size,
        tri, attribute_data_offset, vertex_stride,
        tex.attribute_uv_set_index,
        inv_view_proj,
        os_vertices,
        world_model
    );

    // Convert UV derivatives to texture-space (texels per pixel)
    // U coordinates scale by texture WIDTH, V coordinates scale by texture HEIGHT
    let dudx_texels = d.dudx * f32(tex.size.x);
    let dudy_texels = d.dudy * f32(tex.size.x);
    let dvdx_texels = d.dvdx * f32(tex.size.y);
    let dvdy_texels = d.dvdy * f32(tex.size.y);

    // Compute gradient magnitude (texels per pixel)
    // Standard OpenGL/DirectX formula: rho = max(||dUV/dx||, ||dUV/dy||)
    let rho_x = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);
    let rho_y = sqrt(dudy_texels * dudy_texels + dvdy_texels * dvdy_texels);
    let rho = max(rho_x, rho_y);

    // Compute LOD: LOD = log2(rho) + bias
    var lod = log2(max(rho, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS;

    // Clamp to valid range
    let atlas = get_atlas_info(tex.atlas_index);
    let max_lod = max(atlas.levels_f - 1.0, 0.0);
    lod = clamp(lod, 0.0, max_lod);

    // Apply clamping for out-of-bounds UVs
    let uv_center = texture_uv(attribute_data_offset, tri, vec3<f32>(1.0/3.0, 1.0/3.0, 1.0/3.0), tex, vertex_stride);
    lod = atlas_clamp_cap(lod, tex, uv_center);

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
