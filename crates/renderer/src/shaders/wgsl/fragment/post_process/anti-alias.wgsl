@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var linear_texture_sampler: sampler;
@group(0) @binding(2) var accumulation_texture: texture_2d<f32>;
@group(0) @binding(3) var clip_position_texture_curr: texture_2d<f32>;
@group(0) @binding(4) var clip_position_texture_prev: texture_2d<f32>;


fn anti_alias(in: FragmentInput) -> vec4<f32> {
    // Sample current frame's scene
    let color_curr = textureSample(scene_texture, linear_texture_sampler, in.uv);

    // All textures are the same size 
    let texture_size = vec2<f32>(textureDimensions(scene_texture, 0));
    let pixel_coords = vec2<i32>(in.uv * texture_size);
    //let pixel_coords = vec2<i32>(in.full_screen_quad_position.xy);


    // Get motion vector
    let motion_vec = get_motion_vec(pixel_coords);
        let history_uv = in.uv - motion_vec;

    var history = sample_bicubic(accumulation_texture, linear_texture_sampler, history_uv);
    //var history = textureLoad(accumulation_texture, pixel_coords, 0);

    let near_color_0 = textureLoad(scene_texture, pixel_coords + vec2<i32>(1, 0), 0);
    let near_color_1 = textureLoad(scene_texture, pixel_coords + vec2<i32>(0, 1), 0);
    let near_color_2 = textureLoad(scene_texture, pixel_coords + vec2<i32>(-1, 0), 0);
    let near_color_3 = textureLoad(scene_texture, pixel_coords + vec2<i32>(0, -1), 0);

    let box_min = min(color_curr, min(near_color_0, min(near_color_1, min(near_color_2, near_color_3))));
    let box_max = max(color_curr, max(near_color_0, max(near_color_1, max(near_color_2, near_color_3))));

    history = clamp(history, box_min, box_max);

    return mix(color_curr, history, 0.1);


}

fn anti_alias_old(in: FragmentInput) -> vec4<f32> {
    let color_curr = textureSample(scene_texture, linear_texture_sampler, in.uv);
    return color_curr;
    // Sample current frame's scene
    // let color_curr = textureSample(scene_texture, linear_texture_sampler, in.uv);

    // // All textures are the same size 
    // let texture_size = vec2<f32>(textureDimensions(scene_texture, 0));
    // let pixel_coords = vec2<i32>(in.uv * texture_size);
    // //let pixel_coords = vec2<i32>(in.full_screen_quad_position.xy);


    // // Get motion vector
    // let motion_vec = get_motion_vec(pixel_coords);

    // // Sample history using motion vector
    // let history_uv = in.uv - motion_vec;

    // // Check if history UV is within bounds using WGSL tricks
    // let in_bounds = all(history_uv >= vec2<f32>(0.0)) && all(history_uv <= vec2<f32>(1.0));
    
    // // Sample history color at the reprojected location

    // var color_history = textureSample(accumulation_texture, linear_texture_sampler, history_uv);
    // //var color_history = sample_bicubic(accumulation_texture, linear_texture_sampler, history_uv);
    // //var color_history = sample_catmull_rom(accumulation_texture, linear_texture_sampler, history_uv);


    // // Get neighborhood statistics for clamping
    // // let neighborhood = get_neighborhood_minmax(in.uv);
    // // let color_min = neighborhood.color_min;
    // // let color_max = neighborhood.color_max;
    // //let clamped_history = clip_aabb(color_history, color_min, color_max, color_curr);

    // let neighborhood_stats = compute_neighborhood_stats(in.uv);
    // let neighborhood_mean = neighborhood_stats.rgb;
    // let neighborhood_luminance = neighborhood_stats.w;
    // // Convert current sample to YCoCg for processing
    // let current_ycocg = rgb_to_ycocg(color_curr.rgb);
    // let history_ycocg = rgb_to_ycocg(color_history.rgb);
    // let variance_3d = max(history_variance.rgb, vec3<f32>(0.001));
    // let clamped_history = clip_aabb_v2(neighborhood_mean, neighborhood_luminance, vec4<f32>(history_ycocg, color_history.a));

    // let blend_factor = 0.1;
    // let blend_color = mix(clamped_history, color_curr, blend_factor);
    // return blend_color;
    // // Use current color if out of bounds, otherwise use blended color
    // //return select(color_curr, blend_color, in_bounds);

}



fn get_motion_vec(pixel_coords: vec2<i32>) -> vec2<f32> {
    var pos_curr: vec4<f32> = textureLoad(clip_position_texture_curr, pixel_coords, 0);
    var pos_prev: vec4<f32> = textureLoad(clip_position_texture_prev, pixel_coords, 0);
    // var pos_prev: vec4<f32> = textureLoad(clip_position_texture_prev, pixel_coords, 0);

    // Compute motion vector
    let curr_ndc = pos_curr.xy / pos_curr.w;
    let prev_ndc = pos_prev.xy / pos_prev.w;
    
    // // Convert to UV space and compute velocity
    let curr_uv = curr_ndc * 0.5 + 0.5;
    let prev_uv = prev_ndc * 0.5 + 0.5;
    
    let motion_vec = prev_uv - curr_uv;

    return motion_vec;
}

fn sample_bicubic(tex: texture_2d<f32>, samp: sampler, uv: vec2<f32>) -> vec4<f32> {
    let resolution = vec2<f32>(textureDimensions(tex));
    let sample_pos = uv * resolution - 0.5;
    let tex_pos1 = floor(sample_pos);
    let f = sample_pos - tex_pos1;
    
    // Bicubic weights
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    
    let g0 = w0 + w1;
    let g1 = w2 + w3;
    let h0 = (w1 / g0) - 1.0;
    let h1 = (w3 / g1) + 1.0;
    
    let tex0 = (tex_pos1 + vec2<f32>(h0.x, h0.y)) / resolution;
    let tex1 = (tex_pos1 + vec2<f32>(h1.x, h0.y)) / resolution;
    let tex2 = (tex_pos1 + vec2<f32>(h0.x, h1.y)) / resolution;
    let tex3 = (tex_pos1 + vec2<f32>(h1.x, h1.y)) / resolution;
    
    return g0.y * (g0.x * textureSample(tex, samp, tex0) + g1.x * textureSample(tex, samp, tex1)) +
           g1.y * (g0.x * textureSample(tex, samp, tex2) + g1.x * textureSample(tex, samp, tex3));
}

// Variance-based neighborhood clamping
fn clip_aabb(history: vec4<f32>, color_min: vec4<f32>, color_max: vec4<f32>, current: vec4<f32>) -> vec4<f32> {
    let center = (color_max + color_min) * 0.5;
    let extents = (color_max - color_min) * 0.5;
    
    let offset = history - center;
    let ts = abs(extents / max(abs(offset), vec4<f32>(0.0001)));
    let t = min(min(ts.x, ts.y), min(ts.z, ts.w));
    
    if t < 1.0 {
        return center + offset * t;
    }
    return history;
}

fn clip_aabb_v2(center: vec3<f32>, variance: vec3<f32>, history: vec4<f32>) -> vec4<f32> {
    let std_dev = sqrt(variance);
    let aabb_min = center - std_dev * 1.25;
    let aabb_max = center + std_dev * 1.25;
    
    let clipped = clamp(history.rgb, aabb_min, aabb_max);
    return vec4<f32>(clipped, history.a);
}

struct NeighborhoodMinMax {
    color_min: vec4<f32>,
    color_max: vec4<f32>,
}

fn get_neighborhood_minmax(uv: vec2<f32>) -> NeighborhoodMinMax {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(scene_texture));
    
    var color_min = vec4<f32>(1000.0);
    var color_max = vec4<f32>(-1000.0);
    
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            let neighbor = textureSample(scene_texture, linear_texture_sampler, uv + offset);
            color_min = min(color_min, neighbor);
            color_max = max(color_max, neighbor);
        }
    }
    
    return NeighborhoodMinMax(color_min, color_max);
}

fn sample_catmull_rom(tex: texture_2d<f32>, samp: sampler, uv: vec2<f32>) -> vec4<f32> {
    let resolution = vec2<f32>(textureDimensions(tex));
    let sample_pos = uv * resolution;
    let tex_pos1 = floor(sample_pos - 0.5) + 0.5;
    let f = sample_pos - tex_pos1;
    
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    
    let w12 = w1 + w2;
    let offset12 = w2 / w12;
    
    let tex_pos0 = tex_pos1 - 1.0;
    let tex_pos3 = tex_pos1 + 2.0;
    let tex_pos12 = tex_pos1 + offset12;
    
    let tex_pos0_uv = tex_pos0 / resolution;
    let tex_pos3_uv = tex_pos3 / resolution;
    let tex_pos12_uv = tex_pos12 / resolution;
    
    var result = vec4<f32>(0.0);
    result += textureSample(tex, samp, vec2<f32>(tex_pos0_uv.x, tex_pos0_uv.y)) * w0.x * w0.y;
    result += textureSample(tex, samp, vec2<f32>(tex_pos12_uv.x, tex_pos0_uv.y)) * w12.x * w0.y;
    result += textureSample(tex, samp, vec2<f32>(tex_pos3_uv.x, tex_pos0_uv.y)) * w3.x * w0.y;
    
    result += textureSample(tex, samp, vec2<f32>(tex_pos0_uv.x, tex_pos12_uv.y)) * w0.x * w12.y;
    result += textureSample(tex, samp, vec2<f32>(tex_pos12_uv.x, tex_pos12_uv.y)) * w12.x * w12.y;
    result += textureSample(tex, samp, vec2<f32>(tex_pos3_uv.x, tex_pos12_uv.y)) * w3.x * w12.y;
    
    result += textureSample(tex, samp, vec2<f32>(tex_pos0_uv.x, tex_pos3_uv.y)) * w0.x * w3.y;
    result += textureSample(tex, samp, vec2<f32>(tex_pos12_uv.x, tex_pos3_uv.y)) * w12.x * w3.y;
    result += textureSample(tex, samp, vec2<f32>(tex_pos3_uv.x, tex_pos3_uv.y)) * w3.x * w3.y;
    
    return result;
}

fn compute_neighborhood_stats(uv: vec2<f32>) -> vec4<f32> {
    let inv_resolution = 1.0 / vec2<f32>(textureDimensions(scene_texture));
    var mean = vec3<f32>(0.0);
    var variance = vec3<f32>(0.0);
    var min_color = vec3<f32>(999999.0);
    var max_color = vec3<f32>(-999999.0);
    
    let sample_count = 9.0;
    
    // 3x3 neighborhood
    for (var x: i32 = -1; x <= 1; x++) {
        for (var y: i32 = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * inv_resolution;
            let sample_uv = uv + offset;
            let color = textureSample(scene_texture, linear_texture_sampler, sample_uv).rgb;
            
            // Convert to YCoCg for better temporal stability
            let ycocg_color = rgb_to_ycocg(color);
            
            mean += ycocg_color;
            variance += ycocg_color * ycocg_color;
            min_color = min(min_color, ycocg_color);
            max_color = max(max_color, ycocg_color);
        }
    }
    
    mean /= sample_count;
    variance = variance / sample_count - mean * mean;
    
    // Return mean luminance for adaptive feedback
    return vec4<f32>(mean, luminance_weight(ycocg_to_rgb(mean)));
}

// Luminance-weighted color distance
fn luminance_weight(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.299, 0.587, 0.114));
}

// Color space conversion for better temporal stability
fn rgb_to_ycocg(color: vec3<f32>) -> vec3<f32> {
    let y = dot(color, vec3<f32>(0.25, 0.5, 0.25));
    let co = dot(color, vec3<f32>(0.5, 0.0, -0.5));
    let cg = dot(color, vec3<f32>(-0.25, 0.5, -0.25));
    return vec3<f32>(y, co, cg);
}

fn ycocg_to_rgb(color: vec3<f32>) -> vec3<f32> {
    let y = color.x;
    let co = color.y;
    let cg = color.z;
    
    let r = y + co - cg;
    let g = y + cg;
    let b = y - co - cg;
    
    return vec3<f32>(r, g, b);
}