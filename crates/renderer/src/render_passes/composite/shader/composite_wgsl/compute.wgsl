@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<i32>(global_id.xy);
    let dimensions = textureDimensions(material_tex);

    // Bounds check
    if (coords.x >= i32(dimensions.x) || coords.y >= i32(dimensions.y)) {
        return;
    }

    // Read from input texture
    let material_color = textureLoad(material_tex, coords, 0);

    // TODO - handle OIT

    // Write to output texture
    textureStore(composite_tex, coords, material_color);
}
