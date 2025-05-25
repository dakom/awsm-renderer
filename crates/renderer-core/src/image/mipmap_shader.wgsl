@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let output_size = textureDimensions(output_texture);
    
    if (global_id.x >= output_size.x || global_id.y >= output_size.y) {
        return;
    }
    
    let input_size = textureDimensions(input_texture);
    
    // Calculate the corresponding coordinates in the input texture
    let coord = vec2<f32>(f32(global_id.x), f32(global_id.y));
    let input_coord = (coord + 0.5) * vec2<f32>(f32(input_size.x), f32(input_size.y)) / vec2<f32>(f32(output_size.x), f32(output_size.y)) - 0.5;
    
    // Sample 4 texels and average them (box filter)
    let texel_size = 1.0 / vec2<f32>(f32(input_size.x), f32(input_size.y));
    let offset = texel_size * 0.5;
    
    let sample0 = textureLoad(input_texture, vec2<i32>(input_coord - offset), 0);
    let sample1 = textureLoad(input_texture, vec2<i32>(input_coord + vec2<f32>(offset.x, -offset.y)), 0);
    let sample2 = textureLoad(input_texture, vec2<i32>(input_coord + vec2<f32>(-offset.x, offset.y)), 0);
    let sample3 = textureLoad(input_texture, vec2<i32>(input_coord + offset), 0);
    
    let result = (sample0 + sample1 + sample2 + sample3) * 0.25;
    
    textureStore(output_texture, vec2<i32>(global_id.xy), result);
}