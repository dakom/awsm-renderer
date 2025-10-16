// --- constants & helpers -------------------------------------
const PI      : f32 = 3.1415926535897932384626433832795;
const EPSILON : f32 = 1e-4;

fn saturate(x: f32) -> f32 { return clamp(x, 0.0, 1.0); }
fn saturate3(v: vec3<f32>) -> vec3<f32> { return clamp(v, vec3<f32>(0.0), vec3<f32>(1.0)); }

// attenuation for a point/spot light, matches Unity/Filament
fn inverse_square(range: f32, dist: f32) -> f32 {
    if (range == 0.0) {        // infinite
        return 1.0 / max(dist * dist, 0.01);
    }
    let denom = dist * dist + 1.0;
    let falloff = (1.0 - (dist * dist) / (range * range));
    return saturate(falloff * falloff) / denom;
}
