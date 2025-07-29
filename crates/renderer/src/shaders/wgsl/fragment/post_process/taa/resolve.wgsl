@group(0) @binding(0) var current_color: texture_2d<f32>;
@group(0) @binding(1) var history_color: texture_2d<f32>;
@group(0) @binding(2) var motion_vectors: texture_2d<f32>;
@group(0) @binding(3) var current_depth: texture_2d<f32>;
@group(0) @binding(4) var history_depth: texture_2d<f32>;
@group(0) @binding(5) var history_variance: texture_2d<f32>;
@group(0) @binding(6) var linear_sampler: sampler;

struct TAAUniforms {
    feedback_factor: f32,
    variance_gamma: f32,
    motion_amplification: f32,
    depth_threshold: f32,
    luminance_weight: f32,
    variance_clipping: f32,
    sharpening_factor: f32,
    resolution: vec2<f32>,
    inv_resolution: vec2<f32>,
    frame_index: u32,
    reset_history: u32,
    camera_moved: u32,
    scene_changed: u32,
}

@group(1) @binding(0) var<uniform> taa_uniforms: TAAUniforms;

// Bicubic sampling for high-quality history reconstruction
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
fn clip_aabb_variance(center: vec3<f32>, variance: vec3<f32>, history: vec4<f32>) -> vec4<f32> {
    let std_dev = sqrt(variance);
    let aabb_min = center - std_dev * taa_uniforms.variance_clipping;
    let aabb_max = center + std_dev * taa_uniforms.variance_clipping;
    
    let clipped = clamp(history.rgb, aabb_min, aabb_max);
    return vec4<f32>(clipped, history.a);
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

// Tonemap for HDR stability
fn tonemap_reinhard(color: vec3<f32>) -> vec3<f32> {
    return color / (1.0 + color);
}

fn tonemap_reinhard_inverse(color: vec3<f32>) -> vec3<f32> {
    return color / (1.0 - color);
}

// Advanced disocclusion detection
fn detect_disocclusion(uv: vec2<f32>, motion: vec2<f32>, current_depth: f32, history_depth: f32) -> f32 {
    // Depth-based disocclusion
    let depth_diff = abs(current_depth - history_depth);
    let depth_threshold = taa_uniforms.depth_threshold * (1.0 + current_depth * 10.0);
    let depth_factor = step(depth_threshold, depth_diff);
    
    // Motion-based disocclusion (objects moving too fast)
    let motion_magnitude = length(motion * taa_uniforms.resolution);
    let motion_factor = smoothstep(50.0, 100.0, motion_magnitude);
    
    // Edge-based disocclusion (check if we're near geometric edges)
    let edge_factor = detect_geometric_edge(uv, current_depth);
    
    return max(depth_factor, max(motion_factor, edge_factor));
}

fn detect_geometric_edge(uv: vec2<f32>, center_depth: f32) -> f32 {
    let depth_gradient = vec2<f32>(0.0);
    let offset = taa_uniforms.inv_resolution;
    
    // Sobel edge detection on depth
    let d00 = textureSample(current_depth, linear_sampler, uv + vec2<f32>(-offset.x, -offset.y)).r;
    let d01 = textureSample(current_depth, linear_sampler, uv + vec2<f32>(0.0, -offset.y)).r;
    let d02 = textureSample(current_depth, linear_sampler, uv + vec2<f32>(offset.x, -offset.y)).r;
    let d10 = textureSample(current_depth, linear_sampler, uv + vec2<f32>(-offset.x, 0.0)).r;
    let d12 = textureSample(current_depth, linear_sampler, uv + vec2<f32>(offset.x, 0.0)).r;
    let d20 = textureSample(current_depth, linear_sampler, uv + vec2<f32>(-offset.x, offset.y)).r;
    let d21 = textureSample(current_depth, linear_sampler, uv + vec2<f32>(0.0, offset.y)).r;
    let d22 = textureSample(current_depth, linear_sampler, uv + vec2<f32>(offset.x, offset.y)).r;
    
    let gx = -d00 - 2.0 * d10 - d20 + d02 + 2.0 * d12 + d22;
    let gy = -d00 - 2.0 * d01 - d02 + d20 + 2.0 * d21 + d22;
    
    let edge_strength = sqrt(gx * gx + gy * gy);
    return smoothstep(0.01, 0.05, edge_strength);
}

// Compute neighborhood statistics with improved sampling
fn compute_neighborhood_stats(uv: vec2<f32>) -> vec4<f32> {
    var mean = vec3<f32>(0.0);
    var variance = vec3<f32>(0.0);
    var min_color = vec3<f32>(999999.0);
    var max_color = vec3<f32>(-999999.0);
    
    let sample_count = 9.0;
    
    // 3x3 neighborhood
    for (var x: i32 = -1; x <= 1; x++) {
        for (var y: i32 = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * taa_uniforms.inv_resolution;
            let sample_uv = uv + offset;
            let color = textureSample(current_color, linear_sampler, sample_uv).rgb;
            
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

// Adaptive feedback based on multiple factors
fn compute_adaptive_feedback(
    motion_magnitude: f32,
    disocclusion_factor: f32,
    luminance_change: f32,
    confidence: f32
) -> f32 {
    var feedback = taa_uniforms.feedback_factor;
    
    // Increase feedback (less history) for high motion
    let motion_factor = smoothstep(0.0, 0.1, motion_magnitude);
    feedback = mix(feedback, 0.1, motion_factor);
    
    // Increase feedback for disocclusions
    feedback = mix(feedback, 0.05, disocclusion_factor);
    
    // Increase feedback for large luminance changes
    let luma_factor = smoothstep(0.1, 0.5, luminance_change);
    feedback = mix(feedback, 0.2, luma_factor);
    
    // Decrease feedback for high confidence samples
    feedback = mix(feedback, feedback * 0.5, confidence);
    
    return clamp(feedback, 0.05, 0.95);
}

// Sharpening filter to counteract TAA blur
fn apply_sharpening(center: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    if (taa_uniforms.sharpening_factor <= 0.0) {
        return center;
    }
    
    let offset = taa_uniforms.inv_resolution;
    let north = textureSample(current_color, linear_sampler, uv + vec2<f32>(0.0, -offset.y)).rgb;
    let south = textureSample(current_color, linear_sampler, uv + vec2<f32>(0.0, offset.y)).rgb;
    let east = textureSample(current_color, linear_sampler, uv + vec2<f32>(offset.x, 0.0)).rgb;
    let west = textureSample(current_color, linear_sampler, uv + vec2<f32>(-offset.x, 0.0)).rgb;
    
    let laplacian = north + south + east + west - 4.0 * center;
    return center - taa_uniforms.sharpening_factor * laplacian;
}

struct TAAOutput {
    @location(0) color: vec4<f32>,
    @location(1) variance: vec4<f32>, // Store variance for next frame
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> TAAOutput {
    var output: TAAOutput;
    
    // Early exit if history should be reset
    if (taa_uniforms.reset_history != 0u || taa_uniforms.scene_changed != 0u) {
        let current_sample = textureSample(current_color, linear_sampler, uv);
        output.color = current_sample;
        output.variance = vec4<f32>(0.0);
        return output;
    }
    
    // Sample current frame
    let current_sample = textureSample(current_color, linear_sampler, uv);
    let current_depth = textureSample(current_depth, linear_sampler, uv).r;
    
    // Sample motion vector with confidence
    let motion_data = textureSample(motion_vectors, linear_sampler, uv);
    let motion = motion_data.xy;
    let motion_confidence = motion_data.w;
    
    // Compute reprojected UV
    let history_uv = uv - motion;
    
    // Check bounds and sample history
    if (history_uv.x < 0.0 || history_uv.x > 1.0 || 
        history_uv.y < 0.0 || history_uv.y > 1.0) {
        // Out of bounds - no history available
        output.color = current_sample;
        output.variance = vec4<f32>(0.0);
        return output;
    }
    
    // High-quality history sampling
    let history_sample = sample_bicubic(history_color, linear_sampler, history_uv);
    let history_depth = textureSample(history_depth, linear_sampler, history_uv).r;
    let history_variance = textureSample(history_variance, linear_sampler, history_uv);
    
    // Detect disocclusion
    let motion_magnitude = length(motion * taa_uniforms.resolution);
    let disocclusion_factor = detect_disocclusion(uv, motion, current_depth, history_depth);
    
    // Compute neighborhood statistics
    let neighborhood_stats = compute_neighborhood_stats(uv);
    let neighborhood_mean = neighborhood_stats.rgb;
    let neighborhood_luminance = neighborhood_stats.w;
    
    // Convert current sample to YCoCg for processing
    let current_ycocg = rgb_to_ycocg(current_sample.rgb);
    let history_ycocg = rgb_to_ycocg(history_sample.rgb);
    
    // Variance-based clamping in YCoCg space
    let variance_3d = max(history_variance.rgb, vec3<f32>(0.001)); // Prevent division by zero
    let clamped_history_ycocg = clip_aabb_variance(neighborhood_mean, variance_3d, vec4<f32>(history_ycocg, history_sample.a));
    
    // Detect luminance changes
    let current_luminance = luminance_weight(current_sample.rgb);
    let history_luminance = luminance_weight(history_sample.rgb);
    let luminance_change = abs(current_luminance - history_luminance) / max(current_luminance, 0.001);
    
    // Compute adaptive feedback factor
    let feedback_factor = compute_adaptive_feedback(
        motion_magnitude,
        disocclusion_factor,
        luminance_change,
        motion_confidence
    );
    
    // Temporal blend in YCoCg space
    let blended_ycocg = mix(clamped_history_ycocg.rgb, current_ycocg, feedback_factor);
    
    // Convert back to RGB
    let blended_rgb = ycocg_to_rgb(blended_ycocg);
    
    // Apply sharpening to counteract TAA blur
    let sharpened_rgb = apply_sharpening(blended_rgb, uv);
    
    // Compute new variance for next frame
    let color_diff = current_ycocg - blended_ycocg;
    let new_variance = mix(variance_3d, color_diff * color_diff, 0.1);
    
    // Output final color and variance
    output.color = vec4<f32>(sharpened_rgb, current_sample.a);
    output.variance = vec4<f32>(new_variance, 1.0);
    
    return output;
}
