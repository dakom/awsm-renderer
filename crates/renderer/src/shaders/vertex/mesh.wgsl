// struct VertexInput {
//     @location(0) position: vec3<f32>,
//     @location(1) normal: vec3<f32>,
//     @location(2) uv: vec2<f32>,
// };

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    // @location(0) fragUV: vec2<f32>,
    // @location(1) fragNormal: vec3<f32>,
};

@vertex
fn vert_main() -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(1.0);
    // let modelViewPosition = camera.view * vec4<f32>(input.position, 1.0);
    // output.position = camera.projection * modelViewPosition;
    // output.fragUV = input.uv;
    // output.fragNormal = (camera.view * vec4<f32>(input.normal, 0.0)).xyz;
    return output;
}
