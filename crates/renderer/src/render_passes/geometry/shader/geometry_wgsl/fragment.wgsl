// Fragment input from vertex shader
struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    // same value as screen_position
    @location(1) clip_position: vec4<f32>,
    @location(2) @interpolate(flat) triangle_index: u32,
    @location(3) barycentric: vec2<f32>,  // Full barycentric coordinates
    @location(4) world_normal: vec3<f32>,     // Transformed world-space normal
    @location(5) world_tangent: vec4<f32>,    // Transformed world-space tangent (w = handedness)
}

struct FragmentOutput {
    // Ideally RGBA32Float target, possibly RGBA16Float
    @location(0) visibility_data: vec4<f32>,    // triangle_index, material_offset, bary.xy
    // RG16Float - xy clip coords only (z in depth buffer, w not needed)
    @location(1) taa_clip_position: vec2<f32>,      // Exact clip coords for TAA reprojection
    @location(2) geometry_normal: vec4<f32>,        // xyz = world normal, w unused
    @location(3) geometry_tangent: vec4<f32>,       // xyzw = world tangent (w = handedness)
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    // input.frag_coord already contains screen position + depth!
    // frag_coord.xy = screen coordinates (will inherently exist in compute shader)
    // frag_coord.z = depth (also written to depth buffer)

    // So, compute shader can essentially do:
    // let screen_pos = vec2<f32>(f32(pixel_coord.x), f32(pixel_coord.y));
    // let depth = textureLoad(depth_texture, pixel_coord, 0).x;
    // let world_pos = unproject_screen_to_world(screen_pos, depth, inv_view_proj_matrix);

    var out: FragmentOutput;

    // Pack visibility buffer data
    out.visibility_data = vec4<f32>(
        bitcast<f32>(input.triangle_index),
        bitcast<f32>(mesh_meta.material_offset),
        input.barycentric.x,
        input.barycentric.y  // z = 1.0 - x - y
    );

    // Store exact clip position for TAA (only xy needed, z in depth buffer)
    out.taa_clip_position = input.clip_position.xy;

    // Store transformed world-space normal and tangent
    out.geometry_normal = vec4<f32>(normalize(input.world_normal), 0.0);
    out.geometry_tangent = vec4<f32>(normalize(input.world_tangent.xyz), input.world_tangent.w);

    return out;
}
