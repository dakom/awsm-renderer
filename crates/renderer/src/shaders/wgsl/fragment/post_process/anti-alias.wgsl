@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var linear_texture_sampler: sampler;
@group(0) @binding(2) var accumulation_texture: texture_2d<f32>;
@group(0) @binding(3) var clip_position_texture_curr: texture_2d<f32>;
@group(0) @binding(4) var clip_position_texture_prev: texture_2d<f32>;

fn anti_alias(in: FragmentInput) -> vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(scene_texture, 0));
    let pixel_coords = vec2<i32>(in.uv * texture_size);
    let camera_moved = post_process_data.camera_moved != 0u;

    let color_curr = textureSample(scene_texture, linear_texture_sampler, in.uv);

    let base_blend = select(0.02, 0.08, camera_moved);
    let motion_boost = select(0.08, 0.12, camera_moved); // Different motion response
    let luma_boost = select(0.10, 0.15, camera_moved);   // Different luma response

    // Calculate motion vector and reject if too large
    let motion_vec = get_motion_vec(pixel_coords);
    let motion_length = length(motion_vec);
    
    let history_uv = in.uv - motion_vec;
    let in_bounds = all(history_uv >= vec2<f32>(0.0)) && all(history_uv <= vec2<f32>(1.0));
    let motion_valid = motion_length <= 0.05;
    
    // ALWAYS sample - move outside conditionals
    var color_history = textureSample(accumulation_texture, linear_texture_sampler, history_uv);

    // Neighborhood clamping
    let texel_size = 1.0 / texture_size;
    let samples = array<vec4<f32>, 9>(
        textureSample(scene_texture, linear_texture_sampler, in.uv + vec2<f32>(-1.0, -1.0) * texel_size),
        textureSample(scene_texture, linear_texture_sampler, in.uv + vec2<f32>( 0.0, -1.0) * texel_size),
        textureSample(scene_texture, linear_texture_sampler, in.uv + vec2<f32>( 1.0, -1.0) * texel_size),
        textureSample(scene_texture, linear_texture_sampler, in.uv + vec2<f32>(-1.0,  0.0) * texel_size),
        color_curr,
        textureSample(scene_texture, linear_texture_sampler, in.uv + vec2<f32>( 1.0,  0.0) * texel_size),
        textureSample(scene_texture, linear_texture_sampler, in.uv + vec2<f32>(-1.0,  1.0) * texel_size),
        textureSample(scene_texture, linear_texture_sampler, in.uv + vec2<f32>( 0.0,  1.0) * texel_size),
        textureSample(scene_texture, linear_texture_sampler, in.uv + vec2<f32>( 1.0,  1.0) * texel_size)
    );
    
    var color_min = samples[0];
    var color_max = samples[0];
    for (var i = 1; i < 9; i++) {
        color_min = min(color_min, samples[i]);
        color_max = max(color_max, samples[i]);
    }
    
    color_history = clamp(color_history, color_min, color_max);

    // Adaptive blending
    let luma_curr = dot(color_curr.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let luma_history = dot(color_history.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let luma_diff = abs(luma_curr - luma_history);

    // In your shader, reduce these blend factors for more smoothing:
    var blend_factor = base_blend;
    blend_factor = mix(blend_factor, motion_boost, saturate(motion_length * 200.0));
    blend_factor = mix(blend_factor, luma_boost, saturate(luma_diff * 10.0));

    let frame_weight = saturate(f32(post_process_data.frame_count) / 12.0); // Slower convergence
    blend_factor = mix(0.15, blend_factor, frame_weight); // Lower initial blend
    
    
    let taa_result = mix(color_history, color_curr, blend_factor);
    
    // Sharpening
    let edge_strength = length(vec2<f32>(
        dot(samples[5].rgb - samples[3].rgb, vec3<f32>(0.299, 0.587, 0.114)),
        dot(samples[7].rgb - samples[1].rgb, vec3<f32>(0.299, 0.587, 0.114))
    ));

    let sharpening_strength = saturate(edge_strength * 2.0) * 0.04; // Was 0.08
    
    let center = color_curr;
    let neighbors = (samples[1] + samples[3] + samples[5] + samples[7]) * 0.25;
    let sharpened = center + (center - neighbors) * sharpening_strength;
    
    let final_result = mix(taa_result, sharpened, 0.08); // Was 0.15
    
    let use_taa = in_bounds && motion_valid;


    // //DEBUG: Visualize accumulation strength as colors
    // let accumulation_strength = 1.0 - blend_factor;
    
    // if accumulation_strength > 0.9 {
    //     return vec4<f32>(0.0, 1.0, 0.0, 1.0); // Green = high accumulation
    // } else if accumulation_strength > 0.8 {
    //     return vec4<f32>(1.0, 1.0, 0.0, 1.0); // Yellow = medium accumulation  
    // } else {
    //     return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Red = low accumulation
    // }


    // Use select for conditional return - keeps uniform control flow
    return select(color_curr, final_result, use_taa);
}

fn get_motion_vec(pixel_coords: vec2<i32>) -> vec2<f32> {
    let pos_curr = textureLoad(clip_position_texture_curr, pixel_coords, 0);
    let pos_prev = textureLoad(clip_position_texture_prev, pixel_coords, 0);

    // Convert to NDC
    let curr_ndc = pos_curr.xy / pos_curr.w;
    let prev_ndc = pos_prev.xy / pos_prev.w;
    
    // Convert to UV space
    let curr_uv = curr_ndc * 0.5 + 0.5;
    let prev_uv = prev_ndc * 0.5 + 0.5;
    
    return curr_uv - prev_uv;
}