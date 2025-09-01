// Fragment input from vertex shader
struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec3<f32>, 
    @location(1) clip_position: vec4<f32>,
    @location(2) @interpolate(flat) triangle_id: u32,
    @location(3) barycentric: vec3<f32>,  // Full barycentric coordinates
}

// Output to triangle data texture
struct FragmentOutput {
    //@location(0) triangle_data: vec4<u32>,  // triangle_id, barycentric_xy packed, material_id
    @location(0) material_offset: u32,
    @location(1) world_normal: vec4<f32>,
    @location(2) screen_pos: vec4<f32>,
    @location(3) motion_vector: vec2<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;
    
    // Pack triangle data for the visibility buffer
    // Format: [triangle_id, barycentric_packed, material_id, unused]
    
    // Pack barycentric coordinates into a single u32
    // We can pack two f32 barycentrics (x, y) since z = 1.0 - x - y
    let bary_x_packed = u32(input.barycentric.x * 65535.0); // 16 bits
    let bary_y_packed = u32(input.barycentric.y * 65535.0); // 16 bits
    let barycentric_packed = (bary_x_packed << 16u) | bary_y_packed;
    
    // TODO: Get actual material ID from triangle data buffer
    let material_id = 0u;
    
    // out.triangle_data = vec4<u32>(
    //     input.triangle_id,
    //     barycentric_packed,
    //     material_id,
    //     0u  // Unused - could be used for other data
    // );

    out.material_offset = 1u;
    
    return out;
}