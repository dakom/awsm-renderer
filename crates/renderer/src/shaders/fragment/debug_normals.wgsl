struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    {% if has_normals %}
        @location(0) normal: vec3<f32>,
    {% endif %}
};

@fragment
fn frag_main(input: FragmentInput) -> @location(0) vec4<f32> {
    {% if has_normals %}
        let normal    = normalize(input.normal);
        var rgb_color = normal * 0.5 + vec3<f32>(0.5);
    {% else %}
        var rgb_color = vec3<f32>(1.0, 1.0, 1.0);
    {% endif %}

    return vec4(rgb_color, 1.0);
}
