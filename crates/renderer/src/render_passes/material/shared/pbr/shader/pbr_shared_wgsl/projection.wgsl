/// Unprojects screen coordinates and depth back to world space position
/// screen_pos: pixel coordinates (e.g., from compute shader dispatch coordinates)
/// depth: normalized depth value from depth buffer [0.0, 1.0]
/// screen_size: viewport dimensions (width, height)
/// inv_view_proj: inverse of view-projection matrix
fn unproject_screen_to_world(
    screen_pos: vec2<f32>, 
    depth: f32, 
    screen_size: vec2<f32>,
    inv_view_proj: mat4x4<f32>
) -> vec3<f32> {
    // Convert screen coordinates to normalized device coordinates (NDC)
    // Screen space: [0, width] x [0, height] 
    // NDC space: [-1, 1] x [-1, 1]
    let ndc_x = (screen_pos.x / screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_pos.y / screen_size.y) * 2.0;  // Flip Y for NDC
    
    // Reconstruct clip space position
    // Note: depth is already in [0, 1] range for WebGPU
    let clip_pos = vec4<f32>(ndc_x, ndc_y, depth, 1.0);
    
    // Transform from clip space to world space
    let world_pos_homogeneous = inv_view_proj * clip_pos;
    
    // Perspective divide to get final world position
    return world_pos_homogeneous.xyz / world_pos_homogeneous.w;
}
