fn unlit(color: PbrMaterialColor) -> vec3<f32> {
    // Per glTF KHR_materials_unlit extension:
    // Unlit materials are not affected by lighting - output base color directly
    // Emissive is added on top (though typically unlit materials don't use emissive)
    return color.base.rgb + color.emissive;
}
