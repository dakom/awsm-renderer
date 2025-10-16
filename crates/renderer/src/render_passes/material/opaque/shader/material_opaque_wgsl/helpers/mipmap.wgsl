// Rebuild analytic texture gradients so the compute pass matches fixed-function rasterization.
// The high-level plan:
//   1. Project the triangle's vertices back into screen space.
//   2. Fit planes for (u/w), (v/w), and (1/w) across the triangle footprint.
//   3. Apply the quotient rule to obtain ∂u/∂x, ∂u/∂y, ∂v/∂x, ∂v/∂y.
//   4. Fold those derivatives into the same mip bias + clamp logic we used before.
// This keeps derivatives deterministic (no neighbour fetches), avoids contention on the visibility
// texture, and produces exactly the mip decisions a fragment shader would make.

// Nudges the sampled mip slightly sharper to compensate for historical bias in our shading path.
// Keeping this identical to the fragment implementation avoids visual divergence when we switch
// between pipelines.
const MIPMAP_GLOBAL_LOD_BIAS: f32 = -0.5;

// Atlas padding and clamp epsilon must stay in sync with `MegaTexture::new` on the CPU.
const MIPMAP_ATLAS_PADDING: f32 = 8.0;
const MIPMAP_CLAMP_EPSILON: f32 = 1e-4;

// Small determinant threshold: protects the plane fit from nearly degenerate (or clipped) data.
const MIPMAP_MIN_DET: f32 = 1e-6;

// Represents a fitted plane value(x,y) = base + dx*x + dy*y. The `valid` flag lets the caller bail
// out when the triangle was too thin to recover reliable derivatives.
struct PlaneCoefficients {
    base: f32,
    dx: f32,
    dy: f32,
    valid: bool,
};

// Stores the projected screen position and 1/w for a vertex. When projection fails (clip.w ~ 0) we
// mark the vertex invalid so the mip computation falls back to a safe value.
struct VertexProjection {
    screen: vec2<f32>,
    inv_w: f32,
    valid: bool,
};

// Evaluate every texture the material references. Each texture may have different atlas bounds or
// sampler state, so we still run the final clamp/bias logic per texture even though UV derivatives
// are shared.
fn pbr_get_mipmap_levels(
    pbr_material: PbrMaterial,
    coords: vec2<i32>,
    triangle_index: u32,
    barycentric: vec3<f32>,
    attribute_indices_offset: u32,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    screen_dims_i32: vec2<i32>,
    model_transform: mat4x4<f32>,
) -> PbrMaterialMipLevels {
    let triangle_indices_current = get_triangle_indices(attribute_indices_offset, triangle_index);
    // Convert to float so we can work in pixel space without recasting every time.
    let screen_dims = vec2<f32>(f32(screen_dims_i32.x), f32(screen_dims_i32.y));
    // Shade at the pixel centre; this mirrors the sample position used by hardware derivatives.
    let pixel_center = vec2<f32>(f32(coords.x) + 0.5, f32(coords.y) + 0.5);

    let base_color_lod = compute_texture_mipmap_lod(
        pbr_material.base_color_tex_info,
        triangle_indices_current,
        barycentric,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims,
        pixel_center,
        model_transform,
        pbr_material.has_base_color_texture,
    );

    let metallic_roughness_lod = compute_texture_mipmap_lod(
        pbr_material.metallic_roughness_tex_info,
        triangle_indices_current,
        barycentric,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims,
        pixel_center,
        model_transform,
        pbr_material.has_metallic_roughness_texture,
    );

    let normal_lod = compute_texture_mipmap_lod(
        pbr_material.normal_tex_info,
        triangle_indices_current,
        barycentric,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims,
        pixel_center,
        model_transform,
        pbr_material.has_normal_texture,
    );

    let occlusion_lod = compute_texture_mipmap_lod(
        pbr_material.occlusion_tex_info,
        triangle_indices_current,
        barycentric,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims,
        pixel_center,
        model_transform,
        pbr_material.has_occlusion_texture,
    );

    let emissive_lod = compute_texture_mipmap_lod(
        pbr_material.emissive_tex_info,
        triangle_indices_current,
        barycentric,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims,
        pixel_center,
        model_transform,
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

// Calculate the mip level for a single texture. The derivative reconstruction is shared, but each
// texture needs its own texel-space scaling and sampler clamp rules.
fn compute_texture_mipmap_lod(
    tex_info: TextureInfo,
    triangle_indices_current: vec3<u32>,
    barycentric: vec3<f32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    screen_dims: vec2<f32>,
    pixel_center: vec2<f32>,
    model_transform: mat4x4<f32>,
    texture_enabled: bool,
) -> f32 {
    if (!texture_enabled) {
        return 0.0;
    }

    // Interpolated UV for clamp tests and eventual sampling.
    let uv_center = texture_uv(
        attribute_data_offset,
        triangle_indices_current,
        barycentric,
        tex_info,
        vertex_attribute_stride,
    );

    // Raw per-vertex UVs; needed to fit the analytic plane.
    let uv0 = _texture_uv_per_vertex(
        attribute_data_offset,
        tex_info.attribute_uv_set_index,
        triangle_indices_current.x,
        vertex_attribute_stride,
    );
    let uv1 = _texture_uv_per_vertex(
        attribute_data_offset,
        tex_info.attribute_uv_set_index,
        triangle_indices_current.y,
        vertex_attribute_stride,
    );
    let uv2 = _texture_uv_per_vertex(
        attribute_data_offset,
        tex_info.attribute_uv_set_index,
        triangle_indices_current.z,
        vertex_attribute_stride,
    );

    // Fetch positions so we can re-project the triangle into clip space. Positions are stored at
    // the start of the attribute stream; the shader template supplies the offset constants.
    let pos0 = get_vertex_position(
        attribute_data_offset,
        triangle_indices_current.x,
        vertex_attribute_stride,
    );
    let pos1 = get_vertex_position(
        attribute_data_offset,
        triangle_indices_current.y,
        vertex_attribute_stride,
    );
    let pos2 = get_vertex_position(
        attribute_data_offset,
        triangle_indices_current.z,
        vertex_attribute_stride,
    );

    // Project vertices back into pixel space. If any vertex fails the perspective divide we skip
    // the mip computation; the visibility pipeline should have clipped the triangle already.
    let proj0 = project_vertex(pos0, model_transform, screen_dims);
    let proj1 = project_vertex(pos1, model_transform, screen_dims);
    let proj2 = project_vertex(pos2, model_transform, screen_dims);
    let projections_valid = proj0.valid && proj1.valid && proj2.valid;
    if (!projections_valid) {
        return 0.0;
    }

    // Prepare the values we want the plane to match. These mirror the algebra performed by the
    // hardware interpolator stage.
    let u_over_w0 = uv0.x * proj0.inv_w;
    let u_over_w1 = uv1.x * proj1.inv_w;
    let u_over_w2 = uv2.x * proj2.inv_w;

    let v_over_w0 = uv0.y * proj0.inv_w;
    let v_over_w1 = uv1.y * proj1.inv_w;
    let v_over_w2 = uv2.y * proj2.inv_w;

    // Fit planes for (u/w), (v/w), and (1/w). These coefficients give us the numerators and
    // denominators needed for the quotient rule.
    let plane_u_over_w = fit_plane(proj0.screen, proj1.screen, proj2.screen, u_over_w0, u_over_w1, u_over_w2);
    let plane_v_over_w = fit_plane(proj0.screen, proj1.screen, proj2.screen, v_over_w0, v_over_w1, v_over_w2);
    let plane_inv_w = fit_plane(proj0.screen, proj1.screen, proj2.screen, proj0.inv_w, proj1.inv_w, proj2.inv_w);
    let planes_valid = plane_u_over_w.valid && plane_v_over_w.valid && plane_inv_w.valid;
    if (!planes_valid) {
        return 0.0;
    }

    // Evaluate the planes at the pixel centre, then apply the quotient rule:
    //   d(u) = (d(u/w) * (1/w) - (u/w) * d(1/w)) / (1/w)^2
    let A = eval_plane(plane_u_over_w, pixel_center);
    let B = eval_plane(plane_inv_w, pixel_center);
    let C = eval_plane(plane_v_over_w, pixel_center);
    let safe_B = max(abs(B), 1e-6);
    let denom = safe_B * safe_B;

    let dudx = (plane_u_over_w.dx * B - A * plane_inv_w.dx) / denom;
    let dudy = (plane_u_over_w.dy * B - A * plane_inv_w.dy) / denom;
    let dvdx = (plane_v_over_w.dx * B - C * plane_inv_w.dx) / denom;
    let dvdy = (plane_v_over_w.dy * B - C * plane_inv_w.dy) / denom;

    // Promote to texel space so we can build the standard mip lambda.
    let tex_scale = vec2<f32>(f32(tex_info.size.x), f32(tex_info.size.y));
    let dudx_tex = dudx * tex_scale.x;
    let dudy_tex = dudy * tex_scale.x;
    let dvdx_tex = dvdx * tex_scale.y;
    let dvdy_tex = dvdy * tex_scale.y;

    // Match the usual hardware reduction: pick the dominant axis, compute the magnitude, and
    // convert to log2 to get the mip level.
    let rho_x = sqrt(dudx_tex * dudx_tex + dvdx_tex * dvdx_tex);
    let rho_y = sqrt(dudy_tex * dudy_tex + dvdy_tex * dvdy_tex);
    let gradient = max(rho_x, rho_y);
    let lod = log2(max(gradient, 1e-6));
    let max_mip = log2(max(f32(tex_info.size.x), f32(tex_info.size.y)));

    var clamped_lod = clamp(lod, 0.0, max_mip);

    // Preserve the atlas clamp cap: when a sampler clamps to edge we only allow mip levels that
    // stay inside the padded region.
    let clamp_u = tex_info.address_mode_u == ADDRESS_MODE_CLAMP_TO_EDGE;
    let clamp_v = tex_info.address_mode_v == ADDRESS_MODE_CLAMP_TO_EDGE;
    let oob_u = clamp_u && (uv_center.x < -MIPMAP_CLAMP_EPSILON || uv_center.x > 1.0 + MIPMAP_CLAMP_EPSILON);
    let oob_v = clamp_v && (uv_center.y < -MIPMAP_CLAMP_EPSILON || uv_center.y > 1.0 + MIPMAP_CLAMP_EPSILON);
    if (oob_u || oob_v) {
        let max_clamp_lod = log2(max(MIPMAP_ATLAS_PADDING - 1.0, 1.0));
        clamped_lod = min(clamped_lod, max_clamp_lod);
    }

    // Apply the global bias so the mega-texture filtering matches our legacy fragment path.
    clamped_lod = clamp(clamped_lod + MIPMAP_GLOBAL_LOD_BIAS, 0.0, max_mip);

    return clamped_lod;
}

// Attribute layout helper: positions are packed first, so we add the templated offset supplied by
// the shader generator.
fn get_vertex_position(attribute_data_offset: u32, vertex_index: u32, vertex_attribute_stride: u32) -> vec3<f32> {
    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride) + ATTRIBUTE_POSITION_OFFSET;
    return vec3<f32>(
        attribute_data[vertex_start],
        attribute_data[vertex_start + 1u],
        attribute_data[vertex_start + 2u],
    );
}

// Project a vertex into screen space. Returning both the pixel position and 1/w keeps the math
// close to what the fixed-function pipeline operates on.
fn project_vertex(position: vec3<f32>, model_transform: mat4x4<f32>, screen_dims: vec2<f32>) -> VertexProjection {
    let world = model_transform * vec4<f32>(position, 1.0);
    let clip = camera.view_proj * world;
    let abs_w = abs(clip.w);
    if (abs_w < MIPMAP_MIN_DET) {
        return VertexProjection(vec2<f32>(0.0, 0.0), 0.0, false);
    }

    let inv_w = 1.0 / clip.w;
    let ndc = clip.xy * inv_w;
    // Convert NDC into pixel coordinates. Y flips to match the top-left origin used by the compute
    // pass texture addressing.
    let uv = vec2<f32>(
        (ndc.x + 1.0) * 0.5,
        (1.0 - ndc.y) * 0.5,
    );
    let screen = uv * screen_dims;

    return VertexProjection(screen, inv_w, true);
}

// Fit a plane through three points. We use the closed-form solution so the output matches the
// barycentric plane that hardware interpolation would produce.
fn fit_plane(
    p0: vec2<f32>,
    p1: vec2<f32>,
    p2: vec2<f32>,
    v0: f32,
    v1: f32,
    v2: f32,
) -> PlaneCoefficients {
    let det = p0.x * (p1.y - p2.y) + p1.x * (p2.y - p0.y) + p2.x * (p0.y - p1.y);
    if (abs(det) < MIPMAP_MIN_DET) {
        return PlaneCoefficients(0.0, 0.0, 0.0, false);
    }

    let inv_det = 1.0 / det;
    let base = (v0 * (p1.x * p2.y - p2.x * p1.y) +
                v1 * (p2.x * p0.y - p0.x * p2.y) +
                v2 * (p0.x * p1.y - p1.x * p0.y)) * inv_det;
    let dx = (v0 * (p1.y - p2.y) + v1 * (p2.y - p0.y) + v2 * (p0.y - p1.y)) * inv_det;
    let dy = (v0 * (p2.x - p1.x) + v1 * (p0.x - p2.x) + v2 * (p1.x - p0.x)) * inv_det;

    return PlaneCoefficients(base, dx, dy, true);
}

// Evaluate the plane at an arbitrary pixel. Keeping this helper separate keeps the main derivative
// code easy to read.
fn eval_plane(plane: PlaneCoefficients, point: vec2<f32>) -> f32 {
    return plane.base + plane.dx * point.x + plane.dy * point.y;
}
