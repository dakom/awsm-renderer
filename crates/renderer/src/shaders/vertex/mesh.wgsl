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
    var output: VertexOutput; 
    // let pos = array(
    //       vec2f( 0.0,  0.5),  // top center
    //       vec2f(-0.5, -0.5),  // bottom left
    //       vec2f( 0.5, -0.5)   // bottom right
    //     );

    let pos = array(
          vec3f(0.0, 0.0, 0.0),  // bottom left
          vec3f( 1.0, 0.0, 0.0),   // bottom right
          vec3f( 0.0,  1.0, 0.0), // top center 
        );

    // // 0,0 
    // // 1,0
    // // 0,1


    output.position = vec4f(input.position, 1.0);
    //output.position = vec4f(pos[input.vertexIndex], 1.0);

    return output;
}

// @vertex
// fn vert_main(@builtin(vertex_index) vertexIndex : u32) -> @builtin(position) vec4f{
//     let pos = array(
//           vec2f( 0.0,  0.5),  // top center
//           vec2f(-0.5, -0.5),  // bottom left
//           vec2f( 0.5, -0.5)   // bottom right
//         );
//     return vec4f(pos[vertexIndex], 0.0, 1.0);
// }
    // var output:VertexOutput;

    // //output.position = vec4f(input.position, 1.0);
    // output.position = vec4f(pos[input.vertexIndex], 0.0, 1.0);
    // // output.position = vec4<f32>(1.0);
    // // let modelViewPosition = camera.view * vec4<f32>(input.position, 1.0);
    // // output.position = camera.projection * modelViewPosition;
    // // output.fragUV = input.uv;
    // // output.fragNormal = (camera.view * vec4<f32>(input.normal, 0.0)).xyz;
    // return output;
//}
