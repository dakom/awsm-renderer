// Fragment input from vertex shader
struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,     // Transformed world-space normal
    @location(1) world_tangent: vec4<f32>,    // Transformed world-space tangent (w = handedness)
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
