@fragment
fn frag_main(input: FragmentInput) -> @location(0) vec4<f32> {
    {% if material.has_normal %}
        let normal    = normalize(input.normal);
        var rgb_color = normal * 0.5 + vec3<f32>(0.5);
        var rgba_color = vec4<f32>(rgb_color, 1.0);
    {% else %}
        var rgba_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    {% endif %}

    return rgba_color;
}
