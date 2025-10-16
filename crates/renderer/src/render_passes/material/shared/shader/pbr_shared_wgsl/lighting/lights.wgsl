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
    // for now, hardcode directional light

    if index == 0u {
        // Main light: from above-right for good specular highlights
        return Light(
            1u,
            vec3<f32>(1.0, 1.0, 1.0),   // Pure white
            1.0,
            vec3<f32>(5.0, 8.0, 3.0),   // Above and to the right
            100.0,
            vec3<f32>(-0.5, -0.8, -0.3), // Pointing toward origin
            0.9,
            0.85,
        );
    } else {
        // Side light: to illuminate normals and reduce harsh shadows
        return Light(
            1u,
            vec3<f32>(1.0, 1.0, 1.0),   // Pure white
            0.6,
            vec3<f32>(-3.0, 2.0, 5.0),  // From the side
            100.0,
            vec3<f32>(0.3, -0.2, -0.5), // Pointing toward origin
            0.9,
            0.85,
        );
    }

    // let offset = index * 16u;
    // return Light(
    //     u32(lights[offset + 0u]),
    //     vec3<f32>(lights[offset + 1u], lights[offset + 2u], lights[offset + 3u]),
    //     lights[offset + 4u],
    //     vec3<f32>(lights[offset + 5u], lights[offset + 6u], lights[offset + 7u]),
    //     lights[offset + 8u],
    //     vec3<f32>(lights[offset + 9u], lights[offset + 10u], lights[offset + 11u]),
    //     lights[offset + 12u],
    //     lights[offset + 13u],
    //     // 14 and 15 are padding
    // );
}

fn light_to_brdf(light:Light, normal: vec3<f32>, world_position: vec3<f32>) -> LightBrdf {
    var light_dir: vec3<f32>;
    var radiance: vec3<f32>;
    var n_dot_l: f32;

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
