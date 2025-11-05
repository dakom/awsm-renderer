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
    @location(2) geometry_normal: vec4<f32>,        // xyz = world normal, w unused
    // RGB16float
    @location(3) geometry_tangent: vec4<f32>,       // xyzw = world tangent (w = handedness)
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

    // Store transformed world-space normal
    out.geometry_normal = vec4<f32>(normalize(input.world_normal), 0.0);

    // Store transformed world-space tangent
    out.geometry_tangent = vec4<f32>(normalize(input.world_tangent.xyz), input.world_tangent.w);

    return out;
}

fn split16(x: u32) -> vec2<u32> {
  let lo = x & 0xFFFFu;
  let hi = x >> 16u;
  return vec2<u32>(lo, hi);
}
