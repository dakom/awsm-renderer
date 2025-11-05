{% include "all_material_shared_wgsl/math.wgsl" %}

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
    // RGBA16uint
    @location(0) visibility_data: vec4<u32>,    // triangle_index and material_offset (each as packed 32)
    // RG16float
    @location(1) barycentric: vec2<f32>,    // bary.xy
    // RGB16float
    @location(2) normal_tangent: vec4<f32>,
    // RGB16float
    @location(3) placeholder_for_derivatives: vec4<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;

    // Pack visibility buffer data
    let t = split16(input.triangle_index);
    let m = split16(mesh_meta.material_offset);
    out.visibility_data = vec4<u32>(
        t.x,t.y,
        m.x,m.y
    );

    // z = 1.0 - x - y
    out.barycentric = input.barycentric;

    // Pack normal and tangent into a single vec4 (RGBA16Float)
    // octahedral normal (2 channels) + tangent angle (1 channel) + handedness sign (1 channel)
    let N = normalize(input.world_normal);
    let T = normalize(input.world_tangent.xyz);
    let s = input.world_tangent.w; // handedness: +1 or -1
    out.normal_tangent = pack_normal_tangent(N, T, s);

    // Placeholder for future barycentric derivatives
    out.placeholder_for_derivatives = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    return out;
}
