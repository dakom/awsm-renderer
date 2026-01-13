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
    // KHR_materials_clearcoat
    clearcoat: f32,              // Clearcoat layer intensity (0.0 = none, 1.0 = full)
    clearcoat_roughness: f32,    // Roughness of clearcoat layer
    clearcoat_normal: vec3<f32>, // Normal for clearcoat layer (may differ from base normal)
    // KHR_materials_sheen
    sheen_color: vec3<f32>,      // Sheen color at grazing angles
    sheen_roughness: f32,        // Sheen roughness (affects sheen lobe width)
};
