// ============================================================================
// SMAA 1x - Simplified Single-Pass Implementation
// ============================================================================
//
// Based on "Subpixel Morphological Anti-Aliasing" by Jorge Jimenez et al.
// This is a simplified single-pass version optimized for:
// - Texture aliasing (what MSAA doesn't catch)
// - Specular aliasing
// - Shader aliasing
//
// Strategy:
// 1. Edge detection using luma contrast (in perceptual/gamma space)
// 2. Pattern-based neighborhood blending
// 3. Sub-pixel edge handling
//
// Performance: ~15-25 ALU ops per pixel (very affordable for post-process)
// ============================================================================

const SMAA_THRESHOLD: f32 = 0.05;          // Edge detection threshold (lower = more sensitive) - aggressive for thin lines
const SMAA_LOCAL_CONTRAST_THRESHOLD: f32 = 0.03;  // Even more sensitive for local contrast
const SMAA_MAX_SEARCH_STEPS: i32 = 8;     // How far to search for edge patterns
const SMAA_CORNER_ROUNDING: f32 = 0.25;   // Corner smoothing amount

fn apply_smaa(color: vec4<f32>, coords: vec2<i32>) -> vec4<f32> {
    let dimensions = textureDimensions(composite_texture);
    let tex_size = vec2<f32>(f32(dimensions.x), f32(dimensions.y));
    let inv_tex_size = vec2<f32>(1.0 / tex_size.x, 1.0 / tex_size.y);

    // Convert to perceptual space for edge detection (humans perceive edges in gamma space, not linear)
    let center_perceptual = linear_to_srgb(color.rgb);
    let center_luma = rgb_to_luma(center_perceptual);

    // Sample neighbors and convert to perceptual space
    let left_luma   = rgb_to_luma(linear_to_srgb(textureLoad(composite_texture, coords + vec2<i32>(-1, 0), 0).rgb));
    let right_luma  = rgb_to_luma(linear_to_srgb(textureLoad(composite_texture, coords + vec2<i32>(1, 0), 0).rgb));
    let top_luma    = rgb_to_luma(linear_to_srgb(textureLoad(composite_texture, coords + vec2<i32>(0, -1), 0).rgb));
    let bottom_luma = rgb_to_luma(linear_to_srgb(textureLoad(composite_texture, coords + vec2<i32>(0, 1), 0).rgb));

    // Sample diagonals for better thin line detection
    let top_left_luma     = rgb_to_luma(linear_to_srgb(textureLoad(composite_texture, coords + vec2<i32>(-1, -1), 0).rgb));
    let top_right_luma    = rgb_to_luma(linear_to_srgb(textureLoad(composite_texture, coords + vec2<i32>(1, -1), 0).rgb));
    let bottom_left_luma  = rgb_to_luma(linear_to_srgb(textureLoad(composite_texture, coords + vec2<i32>(-1, 1), 0).rgb));
    let bottom_right_luma = rgb_to_luma(linear_to_srgb(textureLoad(composite_texture, coords + vec2<i32>(1, 1), 0).rgb));

    // Calculate luma deltas (edge strength)
    let delta_left   = abs(center_luma - left_luma);
    let delta_right  = abs(center_luma - right_luma);
    let delta_top    = abs(center_luma - top_luma);
    let delta_bottom = abs(center_luma - bottom_luma);

    // Calculate diagonal deltas for thin line detection
    let delta_top_left     = abs(center_luma - top_left_luma);
    let delta_top_right    = abs(center_luma - top_right_luma);
    let delta_bottom_left  = abs(center_luma - bottom_left_luma);
    let delta_bottom_right = abs(center_luma - bottom_right_luma);

    // Find maximum edge (strongest contrast) including diagonals
    let max_horizontal = max(delta_left, delta_right);
    let max_vertical = max(delta_top, delta_bottom);
    let max_diagonal = max(max(delta_top_left, delta_top_right), max(delta_bottom_left, delta_bottom_right));
    let max_delta = max(max(max_horizontal, max_vertical), max_diagonal);

    // Early exit if no significant edge
    if (max_delta < SMAA_THRESHOLD) {
        return color;
    }

    // Debug: visualize edge detection (green = edges detected)
    // return vec4<f32>(0.0, 1.0, 0.0, 1.0);

    // Determine edge orientation (including diagonal detection)
    let is_horizontal_edge = max_horizontal > max_vertical;
    let is_diagonal_edge = max_diagonal > max(max_horizontal, max_vertical);

    // Calculate blending weights based on edge pattern
    var weights = vec2<f32>(0.0);
    var blended = color;

    if (is_diagonal_edge) {
        // Diagonal edge - use 4-way blending for better thin line handling
        blended = diagonal_blending(
            coords,
            center_luma,
            top_left_luma, top_right_luma,
            bottom_left_luma, bottom_right_luma,
            delta_top_left, delta_top_right,
            delta_bottom_left, delta_bottom_right
        );
    } else if (is_horizontal_edge) {
        // Horizontal edge - blend vertically
        weights = calculate_blending_weights_horizontal(
            coords,
            center_luma,
            left_luma,
            right_luma,
            top_luma,
            bottom_luma,
            delta_left,
            delta_right
        );
        blended = neighborhood_blending(coords, weights, true);
    } else {
        // Vertical edge - blend horizontally
        weights = calculate_blending_weights_vertical(
            coords,
            center_luma,
            top_luma,
            bottom_luma,
            left_luma,
            right_luma,
            delta_top,
            delta_bottom
        );
        blended = neighborhood_blending(coords, weights, false);
    }

    return blended;
}

fn calculate_blending_weights_horizontal(
    coords: vec2<i32>,
    center: f32,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    delta_left: f32,
    delta_right: f32
) -> vec2<f32> {
    // For horizontal edges, we blend vertically (top/bottom)
    // Weight calculation based on edge pattern and contrast

    let edge_left = delta_left > SMAA_THRESHOLD;
    let edge_right = delta_right > SMAA_THRESHOLD;

    var weight_top = 0.0;
    var weight_bottom = 0.0;

    // Calculate how much to blend with top neighbor
    let top_contrast = abs(center - top);
    if (top_contrast > SMAA_LOCAL_CONTRAST_THRESHOLD) {
        weight_top = saturate(1.0 - top_contrast / SMAA_THRESHOLD);
    }

    // Calculate how much to blend with bottom neighbor
    let bottom_contrast = abs(center - bottom);
    if (bottom_contrast > SMAA_LOCAL_CONTRAST_THRESHOLD) {
        weight_bottom = saturate(1.0 - bottom_contrast / SMAA_THRESHOLD);
    }

    // Normalize weights
    let total = weight_top + weight_bottom;
    if (total > 0.0) {
        weight_top /= total;
        weight_bottom /= total;
    }

    return vec2<f32>(weight_top, weight_bottom) * 0.75; // More aggressive blending for thin lines
}

fn calculate_blending_weights_vertical(
    coords: vec2<i32>,
    center: f32,
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
    delta_top: f32,
    delta_bottom: f32
) -> vec2<f32> {
    // For vertical edges, we blend horizontally (left/right)

    let edge_top = delta_top > SMAA_THRESHOLD;
    let edge_bottom = delta_bottom > SMAA_THRESHOLD;

    var weight_left = 0.0;
    var weight_right = 0.0;

    // Calculate how much to blend with left neighbor
    let left_contrast = abs(center - left);
    if (left_contrast > SMAA_LOCAL_CONTRAST_THRESHOLD) {
        weight_left = saturate(1.0 - left_contrast / SMAA_THRESHOLD);
    }

    // Calculate how much to blend with right neighbor
    let right_contrast = abs(center - right);
    if (right_contrast > SMAA_LOCAL_CONTRAST_THRESHOLD) {
        weight_right = saturate(1.0 - right_contrast / SMAA_THRESHOLD);
    }

    // Normalize weights
    let total = weight_left + weight_right;
    if (total > 0.0) {
        weight_left /= total;
        weight_right /= total;
    }

    return vec2<f32>(weight_left, weight_right) * 0.75; // More aggressive blending for thin lines
}

fn diagonal_blending(
    coords: vec2<i32>,
    center_luma: f32,
    top_left_luma: f32,
    top_right_luma: f32,
    bottom_left_luma: f32,
    bottom_right_luma: f32,
    delta_top_left: f32,
    delta_top_right: f32,
    delta_bottom_left: f32,
    delta_bottom_right: f32
) -> vec4<f32> {
    let center = textureLoad(composite_texture, coords, 0);

    // Calculate adaptive weights for each diagonal based on contrast
    var weight_tl = 0.0;
    var weight_tr = 0.0;
    var weight_bl = 0.0;
    var weight_br = 0.0;

    if (delta_top_left > SMAA_LOCAL_CONTRAST_THRESHOLD) {
        weight_tl = saturate(1.0 - delta_top_left / SMAA_THRESHOLD);
    }
    if (delta_top_right > SMAA_LOCAL_CONTRAST_THRESHOLD) {
        weight_tr = saturate(1.0 - delta_top_right / SMAA_THRESHOLD);
    }
    if (delta_bottom_left > SMAA_LOCAL_CONTRAST_THRESHOLD) {
        weight_bl = saturate(1.0 - delta_bottom_left / SMAA_THRESHOLD);
    }
    if (delta_bottom_right > SMAA_LOCAL_CONTRAST_THRESHOLD) {
        weight_br = saturate(1.0 - delta_bottom_right / SMAA_THRESHOLD);
    }

    let total_weight = weight_tl + weight_tr + weight_bl + weight_br;

    if (total_weight <= 0.0) {
        return center;
    }

    // Normalize weights
    weight_tl /= total_weight;
    weight_tr /= total_weight;
    weight_bl /= total_weight;
    weight_br /= total_weight;

    // Sample diagonal neighbors
    let top_left = textureLoad(composite_texture, coords + vec2<i32>(-1, -1), 0);
    let top_right = textureLoad(composite_texture, coords + vec2<i32>(1, -1), 0);
    let bottom_left = textureLoad(composite_texture, coords + vec2<i32>(-1, 1), 0);
    let bottom_right = textureLoad(composite_texture, coords + vec2<i32>(1, 1), 0);

    // Blend diagonally with moderate strength for thin lines
    let blend_strength = 0.6;
    var result = center;
    result = mix(result, top_left, weight_tl * blend_strength);
    result = mix(result, top_right, weight_tr * blend_strength);
    result = mix(result, bottom_left, weight_bl * blend_strength);
    result = mix(result, bottom_right, weight_br * blend_strength);

    return result;
}

fn neighborhood_blending(
    coords: vec2<i32>,
    weights: vec2<f32>,
    is_horizontal: bool
) -> vec4<f32> {
    let center = textureLoad(composite_texture, coords, 0);

    if (weights.x <= 0.0 && weights.y <= 0.0) {
        return center;
    }

    var result = center;

    if (is_horizontal) {
        // Blend vertically (top/bottom)
        if (weights.x > 0.0) {
            let top = textureLoad(composite_texture, coords + vec2<i32>(0, -1), 0);
            result = mix(result, top, weights.x);
        }
        if (weights.y > 0.0) {
            let bottom = textureLoad(composite_texture, coords + vec2<i32>(0, 1), 0);
            result = mix(result, bottom, weights.y);
        }
    } else {
        // Blend horizontally (left/right)
        if (weights.x > 0.0) {
            let left = textureLoad(composite_texture, coords + vec2<i32>(-1, 0), 0);
            result = mix(result, left, weights.x);
        }
        if (weights.y > 0.0) {
            let right = textureLoad(composite_texture, coords + vec2<i32>(1, 0), 0);
            result = mix(result, right, weights.y);
        }
    }

    return result;
}

// Convert RGB to perceptual luma (Rec. 709)
fn rgb_to_luma(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}
