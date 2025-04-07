struct VertexInput {
    @builtin(vertex_index) vertexIndex : u32,
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    // @location(0) fragUV: vec2<f32>,
    // @location(1) fragNormal: vec3<f32>,
};

@vertex
fn vert_main(input: VertexInput) -> VertexOutput {
    let clip_pos = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.position = clip_pos; 
    return output;
}