struct FragmentInput {
    @builtin(position) position: vec4<f32>,

    {% if has_normals %}
        @location(0) normal: vec3<f32>,
    {% endif %}
};


@fragment
fn frag_main(input: FragmentInput) -> @location(0) vec4<f32> {
    let color = vec4(1.0, 1.0, 1.0, 1.0);

    {% if has_normals %}
        var rgb_color = vec3f(0.0);
        let normal = normalize(input.normal);
        let light_1 = dot(normal, vec3<f32>(0.5, 0.5, 1.0));
        let light_2 = dot(normal, vec3<f32>(-0.5, -0.5, -1.0));
        rgb_color += light_1 * vec3<f32>(1.0, 0.0, 0.0);
        rgb_color += light_2 * vec3<f32>(0.0, 1.0, 0.0);

    {% else %}
        let rgb_color = color.rgb;
    {% endif %}


    return vec4(rgb_color, color.a);
}
