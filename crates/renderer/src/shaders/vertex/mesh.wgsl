@group(1) @binding(0)
var<uniform> u_transform: TransformUniform;

struct TransformUniform {
    model: mat4x4<f32>,
};

struct VertexInput {
    @builtin(vertex_index) vertexIndex : u32,
    @location(0) position: vec3<f32>,
    // #IF normals
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vert_main(input: VertexInput) -> VertexOutput {
    // Transform the vertex position by the model matrix, and then by the view projection matrix
    var pos = u_transform.model * vec4<f32>(input.position, 1.0);
    pos = camera.view_proj * pos;

    // Assign and return final output
    let output = VertexOutput(pos);

    return output;
}