const pi : f32 = 3.1415926535;

fn saturate(x: f32) -> f32 { return clamp(x, 0.0, 1.0); }

// attenuation for a point/spot light, matches Unity/Filament
fn inverse_square(range: f32, dist: f32) -> f32 {
    if (range == 0.0) {        // infinite
        return 1.0 / max(dist * dist, 0.01);
    }
    let denom = dist * dist + 1.0;
    let falloff = (1.0 - (dist * dist) / (range * range));
    return saturate(falloff * falloff) / denom;
}

// spot light mask (smooth edge)
fn spot_falloff(inner_cos: f32, outer_cos: f32, cos_l: f32) -> f32 {
    let smoothed = saturate((cos_l - outer_cos) / (inner_cos - outer_cos));
    return smoothed * smoothed;
}
