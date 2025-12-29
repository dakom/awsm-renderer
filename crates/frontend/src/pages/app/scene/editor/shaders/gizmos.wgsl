@vertex
fn vert_main(@builtin(vertex_index) vertex_index: u32) -> FragmentInput {
    var out: FragmentInput;

    // Generate oversized triangle vertices using bit manipulation
    // Goal: vertex 0→(-1,-1), vertex 1→(3,-1), vertex 2→(-1,3)

    // X coordinate generation:
    // vertex_index: 0 → 0<<1 = 0 → 0&2 = 0 → 0*2-1 = -1 ✓
    // vertex_index: 1 → 1<<1 = 2 → 2&2 = 2 → 2*2-1 = 3  ✓
    // vertex_index: 2 → 2<<1 = 4 → 4&2 = 0 → 0*2-1 = -1 ✓
    let x = f32((vertex_index << 1u) & 2u) * 2.0 - 1.0;

    // Y coordinate generation:
    // vertex_index: 0 → 0&2 = 0 → 0*2-1 = -1 ✓
    // vertex_index: 1 → 1&2 = 0 → 0*2-1 = -1 ✓
    // vertex_index: 2 → 2&2 = 2 → 2*2-1 = 3  ✓
    let y = f32(vertex_index & 2u) * 2.0 - 1.0;

    out.full_screen_quad_position = vec4<f32>(x, y, 0.0, 1.0);

    return out;
}

struct FragmentInput {
    @builtin(position) full_screen_quad_position: vec4<f32>,
}

@fragment
fn frag_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(in.full_screen_quad_position.xy);

    let color = vec4<f32>(
        f32((coords.x / 10) % 2) * 0.1 + 0.45,
        f32((coords.y / 10) % 2) * 0.1 + 0.45,
        0.5,
        0.5
    );

    if coords.x % 10 == 0 || coords.y % 10 == 0 {
        discard;
    }

    return color;
}
