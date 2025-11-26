// Fragment input from vertex shader
struct FragmentInput {
    @builtin(position) screen_position: vec4<f32>,
    // same coordinate space as screen_position, but this is the interpolated
    // vertex shader output, not the hardware-computed fragment position on the screen
    // useful for TAA motion vectors, not using for now though
    @location(1) clip_position: vec4<f32>,
    @location(2) @interpolate(flat) triangle_index: u32,
    @location(3) barycentric: vec2<f32>,  // Full barycentric coordinates
    @location(4) world_normal: vec3<f32>,     // Transformed world-space normal
    @location(5) world_tangent: vec4<f32>,    // Transformed world-space tangent (w = handedness)
}

struct FragmentOutput {
    // Rgba16float
    @location(0) oit_color: vec4<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;

    out.oit_color = vec4<f32>(1.0, 1.0, 1.0, 0.5);

    return out;
}
