// Fragment shader lighting helper for transparent materials
// Applies all enabled lighting (IBL + punctual lights) to a material

// Apply all enabled lighting to a material and return the final color
fn apply_lighting(
    material_color: PbrMaterialColor,
    surface_to_camera: vec3<f32>,
    world_position: vec3<f32>,
    lights_info: LightsInfo,
) -> vec3<f32> {
    var color = vec3<f32>(0.0);

    {% if has_lighting_ibl() %}
        color = brdf_ibl(
            material_color,
            material_color.normal,
            surface_to_camera,
            ibl_filtered_env_tex,
            ibl_filtered_env_sampler,
            ibl_irradiance_tex,
            ibl_irradiance_sampler,
            brdf_lut_tex,
            brdf_lut_sampler,
            lights_info.ibl
        );
    {% endif %}

    {% if has_lighting_punctual() %}
        for(var i = 0u; i < lights_info.n_lights; i = i + 1u) {
            let light_brdf = light_to_brdf(get_light(i), material_color.normal, world_position);
            color += brdf_direct(material_color, light_brdf, surface_to_camera);
        }
    {% endif %}

    return color;
}
