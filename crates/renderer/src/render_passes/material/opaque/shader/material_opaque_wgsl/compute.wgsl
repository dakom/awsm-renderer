@group(0) @binding(0) var material_offset_tex: texture_2d<u32>;
@group(0) @binding(1) var world_normal_tex: texture_2d<f32>;
@group(0) @binding(2) var screen_pos_tex: texture_2d<f32>;
@group(0) @binding(3) var opaque_tex: texture_storage_2d<rgba16float, write>;

// TODO - bind material uniform buffer, load material properties

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<i32>(global_id.xy);
    let dimensions = textureDimensions(opaque_tex);
    
    // Bounds check
    if (coords.x >= i32(dimensions.x) || coords.y >= i32(dimensions.y)) {
        return;
    }

    let material_offset = textureLoad(material_offset_tex, coords, 0);
    let world_normal = textureLoad(world_normal_tex, coords, 0);
    let screen_pos = textureLoad(screen_pos_tex, coords, 0);
    
    let color = vec4<f32>(1.0, 0.0, 0.0, 1.0); // Temp color for testing
    
    // Write to output texture
    textureStore(opaque_tex, coords, color);
}