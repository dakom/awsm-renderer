{% for binding in fragment_buffer_bindings %}
    @group({{ binding.group }}) @binding({{ binding.index }}) var {{ binding.name }}: {{ binding.data_type }};
{% endfor %}


@fragment
fn frag_main(input: FragmentInput) -> @location(0) vec4<f32> {
    {% if material.has_base_color %}
        var rgba_color = textureSample(base_color_tex, base_color_sampler, input.base_color_uv);
    {% else %}
        var rgba_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    {% endif %}

    return rgba_color;
}
