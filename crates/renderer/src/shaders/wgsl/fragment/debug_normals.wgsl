struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,

    // These locations should probably be set dynamically...
    // but they happen to line up fine for now :p
    @location(0) world_position: vec3<f32>, 

    {% if has_normals %}
        @location(1) world_normal: vec3<f32>,
    {% endif %}
};

@fragment
fn frag_main(input: FragmentInput) -> @location(0) vec4<f32> {
    {% if has_normals %}
        let normal    = normalize(input.world_normal);
        var rgb_color = normal * 0.5 + vec3<f32>(0.5);
        var rgba_color = vec4<f32>(rgb_color, 1.0);
    {% else %}
        var rgba_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    {% endif %}

    return rgba_color;
}
