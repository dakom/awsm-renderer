@group(0) @binding(0) var current_color: texture_2d<f32>;
@group(0) @binding(1) var history_color: texture_2d<f32>;
@group(0) @binding(2) var motion_vectors: texture_2d<f32>;
@group(0) @binding(3) var ghosting_mask: texture_storage_2d<r8unorm, write>;

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coord = vec2<i32>(global_id.xy);
    let resolution = vec2<i32>(textureDimensions(current_color));
    
    if (pixel_coord.x >= resolution.x || pixel_coord.y >= resolution.y) {
        return;
    }
    
    let uv = (vec2<f32>(pixel_coord) + 0.5) / vec2<f32>(resolution);
    let motion = textureLoad(motion_vectors, pixel_coord, 0).xy;
    let history_uv = uv - motion;
    
    // Sample current and history colors
    let current = textureLoad(current_color, pixel_coord, 0).rgb;
    let history = textureSample(history_color, linear_sampler, history_uv).rgb;
    
    // Detect ghosting by comparing luminance changes
    let current_luma = dot(current, vec3<f32>(0.299, 0.587, 0.114));
    let history_luma = dot(history, vec3<f32>(0.299, 0.587, 0.114));
    
    let luma_diff = abs(current_luma - history_luma);
    let ghosting_factor = smoothstep(0.05, 0.2, luma_diff);
    
    // Also check for temporal inconsistencies
    let motion_magnitude = length(motion * vec2<f32>(resolution));
    let motion_factor = smoothstep(10.0, 50.0, motion_magnitude);
    
    let final_ghosting = max(ghosting_factor, motion_factor);
    textureStore(ghosting_mask, pixel_coord, vec4<f32>(final_ghosting));
}