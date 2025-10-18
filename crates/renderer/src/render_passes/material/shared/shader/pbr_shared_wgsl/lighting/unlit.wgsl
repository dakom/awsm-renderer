const AMBIENT = vec3<f32>(1.0); // TODO - make this settable, or get from IBL

fn unlit(color: PbrMaterialColor) -> vec3<f32> {
    // Apply occlusion to ambient lighting
    let occluded_ambient = AMBIENT * color.occlusion;

    let metallic = color.metallic_roughness.x;
    let roughness = color.metallic_roughness.y;

    // For metallic materials, reduce diffuse contribution and use base color as specular
    // Non-metallic materials use base color as diffuse
    let dielectric_color = color.base.rgb;
    let metallic_color = color.base.rgb * 0.3; // Metals appear darker without proper reflections

    // Blend between dielectric and metallic behavior
    let material_color = mix(dielectric_color, metallic_color, metallic);

    // Rougher surfaces absorb more ambient light, smoother surfaces reflect more
    let roughness_factor = mix(1.0, 0.7, roughness);
    let ambient_contribution = material_color * occluded_ambient * roughness_factor;

    return color.emissive + ambient_contribution;
}
