fn apply_tone_mapping(color: vec3<f32>) -> vec3<f32> {
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
