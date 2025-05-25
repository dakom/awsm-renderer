@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let output_size = textureDimensions(output_texture);
    
    if (global_id.x >= output_size.x || global_id.y >= output_size.y) {
        return;
    }
    
    // Calculate normalized coordinates for sampling
    let uv = (vec2<f32>(global_id.xy) + 0.5) / vec2<f32>(output_size);
    
    // Sample with linear filtering for better quality
    let result = textureSampleLevel(input_texture, input_sampler, uv, 0.0);
    
    textureStore(output_texture, vec2<i32>(global_id.xy), result);
}