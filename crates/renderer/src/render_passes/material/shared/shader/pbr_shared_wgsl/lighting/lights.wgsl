// though it's set in the storage buffer as a float array with padding
struct Light {
    kind: u32,
    color: vec3<f32>,
    intensity: f32,
    position: vec3<f32>,
    range: f32,
    direction: vec3<f32>,
    inner_cone: f32,
    outer_cone: f32,
};

fn get_light(index: u32) -> Light {
    switch (index) {
        case 0u: { // key light from front-top
            return Light(
                1u,
                vec3<f32>(1.0, 1.0, 1.0),
                3.5,
                vec3<f32>(0.0, 0.0, 0.0),
                0.0,
                normalize(vec3<f32>(0.3, -0.5, -1.0)),
                0.0,
                0.0,
            );
        }
        case 1u: { // fill from back
            return Light(
                1u,
                vec3<f32>(1.0, 1.0, 1.0),
                2.2,
                vec3<f32>(0.0, 0.0, 0.0),
                0.0,
                normalize(vec3<f32>(-0.2, -0.4, 0.9)),
                0.0,
                0.0,
            );
        }
        case 2u: { // rim from side
            return Light(
                1u,
                vec3<f32>(1.0, 1.0, 1.0),
                1.5,
                vec3<f32>(0.0, 0.0, 0.0),
                0.0,
                normalize(vec3<f32>(-1.0, -0.2, 0.2)),
                0.0,
                0.0,
            );
        }
        case 3u: { // bottom bounce
            return Light(
                1u,
                vec3<f32>(1.0, 1.0, 1.0),
                1.2,
                vec3<f32>(0.0, 0.0, 0.0),
                0.0,
                normalize(vec3<f32>(0.1, 0.9, 0.2)),
                0.0,
                0.0,
            );
        }
        default: {
            return Light(
                0u,
                vec3<f32>(0.0),
                0.0,
                vec3<f32>(0.0),
                0.0,
                vec3<f32>(0.0),
                0.0,
                0.0,
            );
        }
    }
}

struct LightBrdf {
    normal: vec3<f32>,
    n_dot_l: f32,
    light_dir: vec3<f32>,
    radiance: vec3<f32>,
};

fn light_to_brdf(light:Light, normal: vec3<f32>, world_position: vec3<f32>) -> LightBrdf {
    var light_dir: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var radiance: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var n_dot_l: f32 = 0.0;

    switch (light.kind) {
        case 0u: {
            // no light, skip
        }
        case 1u: { // Directional
            light_dir = normalize(-light.direction); // light -> surface
            radiance = light.color * light.intensity;
            n_dot_l = max(dot(normal, light_dir), 0.0);
        }
        case 2u: { // Point
            let surface_to_light = light.position - world_position;
            let dist = length(surface_to_light);
            light_dir = surface_to_light / dist; // light -> surface
            let attenuation = inverse_square(light.range, dist);
            radiance = light.color * attenuation;
            n_dot_l = max(dot(normal, light_dir), 0.0);
        }
        case 3u: { // Spot
            let surface_to_light = light.position - world_position;
            let dist = length(surface_to_light);
            light_dir = surface_to_light / dist; // light -> surface
            let cos_l = dot(light_dir, -normalize(light.direction));
            let spot = spot_falloff(light.inner_cone, light.outer_cone, cos_l);
            let attenuation = inverse_square(light.range, dist) * spot;
            radiance = light.color * attenuation;
            n_dot_l = max(dot(normal, light_dir), 0.0);
        }
        default: { // unexpected
        }
    }

    return LightBrdf(
        normal,
        n_dot_l,
        light_dir,
        radiance,
    );
}

// spot light mask (smooth edge)
fn spot_falloff(inner_cos: f32, outer_cos: f32, cos_l: f32) -> f32 {
    let smoothed = saturate((cos_l - outer_cos) / (inner_cos - outer_cos));
    return smoothed * smoothed;
}
