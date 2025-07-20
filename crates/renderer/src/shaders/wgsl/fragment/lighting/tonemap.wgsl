// 1) Add this helper above your frag_main:
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

fn gamma_correct(x: vec3<f32>, gamma: f32) -> vec3<f32> {
    return pow(x, vec3<f32>(1.0 / gamma));
}