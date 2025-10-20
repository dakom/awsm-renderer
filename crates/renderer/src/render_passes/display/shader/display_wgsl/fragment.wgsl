@group(0) @binding(0) var composite_texture: texture_2d<f32>;

struct FragmentInput {
    @builtin(position) full_screen_quad_position: vec4<f32>,
}

@fragment
fn frag_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(in.full_screen_quad_position.xy);

    let color: vec4<f32> = textureLoad(composite_texture, coords, 0);

    let rgb = linear_to_srgb(color.rgb);

    // Apply tone mapping to compress HDR to displayable range
    let mapped = khronos_pbr_neutral_tonemap(rgb);

    return vec4<f32>(mapped, color.a);
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.0031308);
    let low = color * 12.92;
    let high = 1.055 * pow(color, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, color <= cutoff);
}


fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    // Narkowicz 2015 “ACES Filmic Tone Mapping Curve”
    let a: f32 = 2.51;
    let b: f32 = 0.03;
    let c: f32 = 2.43;
    let d: f32 = 0.59;
    let e: f32 = 0.14;
    let num   = x * (a * x + b);
    let denom = x * (c * x + d) + e;
    return clamp(num / denom, vec3<f32>(0.0), vec3<f32>(1.0));
}


fn khronos_pbr_neutral_tonemap(color: vec3<f32>) -> vec3<f32> {
    let startCompression: f32 = 0.8 - 0.04;
    let desaturation: f32 = 0.15;

    let x: f32 = min(color.r, min(color.g, color.b));
    var offset: f32 = 0.04;
    if x < 0.08 {
        offset = x - 6.25 * x * x;
    }
    var result = color - vec3<f32>(offset);

    let peak: f32 = max(result.r, max(result.g, result.b));
    if peak < startCompression {
        return result;
    }

    let d: f32 = 1.0 - startCompression;
    let newPeak: f32 = 1.0 - d * d / (peak + d - startCompression);
    result *= newPeak / peak;

    let g: f32 = 1.0 - 1.0 / (desaturation * (peak - newPeak) + 1.0);
    return mix(result, vec3<f32>(newPeak), g);
}

fn gamma_correct(x: vec3<f32>, gamma: f32) -> vec3<f32> {
    return pow(x, vec3<f32>(1.0 / gamma));
}
