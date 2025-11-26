{% include "utils_wgsl/math.wgsl" %}

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
    // RGBA16float
    @location(2) normal_tangent: vec4<f32>,
    // RGBA16float
    @location(3) barycentric_derivatives: vec4<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;

    // Pack visibility buffer data
    let t = split16(input.triangle_index);
    // see `mesh_meta`, this is not the material material_offset
    // it's the offset of the mesh_meta data in the material *pass*
    let m = split16(mesh_meta.material_offset);
    // it's 16 bits, not u32, but we store as u32 for simplicity
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

    // perspective-correct barycentrics by default:
    let ddx = dpdx(input.barycentric);          // (db1/dx, db2/dx)
    let ddy = dpdy(input.barycentric);          // (db1/dy, db2/dy)

    out.barycentric_derivatives = vec4<f32>(ddx.x, ddy.x, ddx.y, ddy.y);

    return out;
}
