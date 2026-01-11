// Contains the final material properties after sampling all PBR textures.
// IMPORTANT: The 'normal' field contains the perturbed normal (with normal map applied),
// NOT the geometry normal. Always use material_color.normal for lighting calculations!
struct PbrMaterialColor {
    base: vec4<f32>,
    metallic_roughness: vec2<f32>,
    normal: vec3<f32>,  // Perturbed normal from normal mapping (use this for lighting!)
    occlusion: f32,
    emissive: vec3<f32>,
    specular: f32,           // KHR_materials_specular: strength factor (default 1.0)
    specular_color: vec3<f32>, // KHR_materials_specular: F0 color modifier (default white)
    // KHR_materials_ior
    ior: f32,
    // KHR_materials_transmission
    transmission: f32,
    // KHR_materials_volume
    volume_thickness: f32,
    volume_attenuation_distance: f32,
    volume_attenuation_color: vec3<f32>,
};
