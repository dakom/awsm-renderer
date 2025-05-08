@fragment
fn frag_main(input: FragmentInput) -> @location(0) vec4<f32> {
    let normal    = normalize(input.normal);
    var rgb_color = normal * 0.5 + vec3<f32>(0.5);

    return vec4(rgb_color, 1.0);
}
