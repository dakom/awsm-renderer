@group(0) @binding(0) var input_color: texture_2d<f32>;
@group(0) @binding(1) var motion_vectors: texture_2d<f32>;
@group(0) @binding(2) var output_color: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coord = vec2<i32>(global_id.xy);
    let resolution = vec2<i32>(textureDimensions(input_color));
    
    if (pixel_coord.x >= resolution.x || pixel_coord.y >= resolution.y) {
        return;
    }
    
    let uv = (vec2<f32>(pixel_coord) + 0.5) / vec2<f32>(resolution);
    let motion = textureLoad(motion_vectors, pixel_coord, 0).xy;
    
    // Compensate for motion blur by sampling along motion vector
    let motion_samples = 5;
    var accumulated_color = vec3<f32>(0.0);
    
    for (var i = 0; i < motion_samples; i++) {
        let t = f32(i) / f32(motion_samples - 1);
        let sample_uv = uv - motion * t;
        
        if (sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && 
            sample_uv.y >= 0.0 && sample_uv.y <= 1.0) {
            let sample_coord = vec2<i32>(sample_uv * vec2<f32>(resolution));
            accumulated_color += textureLoad(input_color, sample_coord, 0).rgb;
        }
    }
    
    accumulated_color /= f32(motion_samples);
    textureStore(output_color, pixel_coord, vec4<f32>(accumulated_color, 1.0));
}