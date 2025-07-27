@group(0) @binding(0) var current_color: texture_2d<f32>;
@group(0) @binding(1) var history_color: texture_2d<f32>;
@group(0) @binding(2) var motion_vectors: texture_2d<f32>;
@group(0) @binding(3) var current_depth: texture_2d<f32>;
@group(0) @binding(4) var history_depth: texture_2d<f32>;
@group(0) @binding(5) var history_variance: texture_2d<f32>;
@group(0) @binding(6) var output_color: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var output_variance: texture_storage_2d<rgba16float, write>;
@group(0) @binding(8) var linear_sampler: sampler;

@group(1) @binding(0) var<uniform> taa_uniforms: TAAUniforms;

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coord = vec2<i32>(global_id.xy);
    let resolution = vec2<i32>(textureDimensions(current_color));
    
    if (pixel_coord.x >= resolution.x || pixel_coord.y >= resolution.y) {
        return;
    }
    
    let uv = (vec2<f32>(pixel_coord) + 0.5) / vec2<f32>(resolution);
    
    // Same TAA logic as fragment shader but optimized for compute
    let result = perform_taa_resolve(uv);
    
    textureStore(output_color, pixel_coord, result.color);
    textureStore(output_variance, pixel_coord, result.variance);
}