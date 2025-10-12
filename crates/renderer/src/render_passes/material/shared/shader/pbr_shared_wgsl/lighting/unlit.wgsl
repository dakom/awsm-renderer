fn unlit(color: PbrMaterialColor, ambient: vec3<f32>, surface_to_camera: vec3<f32>) -> vec3<f32> {
    return color.emissive + (color.base.rgb * ambient);
}
