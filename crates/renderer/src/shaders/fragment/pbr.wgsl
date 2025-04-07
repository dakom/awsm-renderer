struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>
    //@location(0) in_color: vec4<f32>
};

@fragment
fn frag_main(input: FragmentInput) -> @location(0) vec4<f32> {
    // the vertex shader does a perspective divide of position.xyz by position.w
    // so we need to multiply by the inverse of that to get the original position.xyz
    let inv_w = 1.0 / input.frag_coord.w;

    // then, to get the world position, we need to multiply by the inverse of the view projection matrix
    let world_pos = camera.inv_view_proj * (input.frag_coord * inv_w); 

    // Output the color
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
