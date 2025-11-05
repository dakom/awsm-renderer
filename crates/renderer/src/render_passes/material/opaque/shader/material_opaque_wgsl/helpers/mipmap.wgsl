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

// Compute per-pixel UV derivatives using depth buffer reconstruction
// This is the "correct" solution for deferred rendering - matches fragment shader quality
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
    // Read depth values for 3-pixel stencil (center + right + down)
    let depth_center = textureLoad(depth_tex, coords, 0);
    let depth_x = textureLoad(depth_tex, coords + vec2<i32>(1, 0), 0);
    let depth_y = textureLoad(depth_tex, coords + vec2<i32>(0, 1), 0);

    // Reconstruct world positions
    let world_center = reconstruct_world_position(pixel_center, depth_center, inv_view_proj, screen_size);
    let world_x = reconstruct_world_position(pixel_center + vec2<f32>(1.0, 0.0), depth_x, inv_view_proj, screen_size);
    let world_y = reconstruct_world_position(pixel_center + vec2<f32>(0.0, 1.0), depth_y, inv_view_proj, screen_size);

    // World-space derivatives (per screen pixel)
    let dWorld_dx = world_x - world_center;
    let dWorld_dy = world_y - world_center;

    // Get triangle vertices in world space by transforming object-space positions
    let v0_world = (world_model * vec4<f32>(os_vertices.p0, 1.0)).xyz;
    let v1_world = (world_model * vec4<f32>(os_vertices.p1, 1.0)).xyz;
    let v2_world = (world_model * vec4<f32>(os_vertices.p2, 1.0)).xyz;

    let uv0 = _texture_uv_per_vertex(attribute_data_offset, uv_set_index, tri.x, vertex_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, uv_set_index, tri.y, vertex_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, uv_set_index, tri.z, vertex_stride);

    // Solve for UV derivatives using barycentric interpolation
    // UV(world) = w0*uv0 + w1*uv1 + w2*uv2, where w are barycentric coords
    // We need: dUV/dScreen = dUV/dWorld * dWorld/dScreen

    // Build edge vectors for triangle in world space
    let e01_world = v1_world - v0_world;
    let e02_world = v2_world - v0_world;

    // Build edge vectors for UVs
    let e01_uv = uv1 - uv0;
    let e02_uv = uv2 - uv0;

    // Compute dWorld/dBarycentric (2x3 matrix)
    // Then chain with dBarycentric/dScreen to get dWorld/dScreen
    // Finally invert to get dUV/dScreen

    // Build 3x3 system to solve for barycentric derivatives
    // [e01_world.x  e02_world.x  dWorld_dx.x]   [dw1/dx]   [0]
    // [e01_world.y  e02_world.y  dWorld_dx.y] * [dw2/dx] = [0]
    // [e01_world.z  e02_world.z  dWorld_dx.z]   [dw0/dx]   [0]
    //
    // With constraint: dw0/dx + dw1/dx + dw2/dx = 0
    //
    // This simplifies to solving a 2D system for dw1/dx, dw2/dx
    // Then dw0/dx = -(dw1/dx + dw2/dx)

    // Project onto triangle plane using cross product
    let tri_normal = normalize(cross(e01_world, e02_world));

    // Project world derivatives onto triangle plane
    let dWorld_dx_proj = dWorld_dx - dot(dWorld_dx, tri_normal) * tri_normal;
    let dWorld_dy_proj = dWorld_dy - dot(dWorld_dy, tri_normal) * tri_normal;

    // Solve 2x2 system: [e01 e02] * [dw1; dw2] = dWorld_proj
    // Using Cramer's rule
    let det = e01_world.x * e02_world.y - e01_world.y * e02_world.x;

    if (abs(det) < 1e-8) {
        // Degenerate triangle - return zero derivatives
        return UvDerivs(0.0, 0.0, 0.0, 0.0);
    }

    let inv_det = 1.0 / det;

    // Solve for barycentric derivatives in 2D (XY plane dominant)
    let dw1_dx = (dWorld_dx_proj.x * e02_world.y - dWorld_dx_proj.y * e02_world.x) * inv_det;
    let dw2_dx = (e01_world.x * dWorld_dx_proj.y - e01_world.y * dWorld_dx_proj.x) * inv_det;

    let dw1_dy = (dWorld_dy_proj.x * e02_world.y - dWorld_dy_proj.y * e02_world.x) * inv_det;
    let dw2_dy = (e01_world.x * dWorld_dy_proj.y - e01_world.y * dWorld_dy_proj.x) * inv_det;

    // UV derivatives: dUV/dScreen = dw1/dScreen * e01_uv + dw2/dScreen * e02_uv
    let dudx = dw1_dx * e01_uv.x + dw2_dx * e02_uv.x;
    let dudy = dw1_dy * e01_uv.x + dw2_dy * e02_uv.x;
    let dvdx = dw1_dx * e01_uv.y + dw2_dx * e02_uv.y;
    let dvdy = dw1_dy * e01_uv.y + dw2_dy * e02_uv.y;

    return UvDerivs(dudx, dudy, dvdx, dvdy);
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

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
