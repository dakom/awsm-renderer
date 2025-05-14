struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>, 

    {% for loc in vertex_output_locations %}
        {%- match loc.interpolation %}
            {% when Some with (interpolation) %}
                @location({{ loc.location }}) @interpolate({{ interpolation }}) {{ loc.name }}: {{ loc.data_type }},
            {% when _ %}
                @location({{ loc.location }}) {{ loc.name }}: {{ loc.data_type }},
        {% endmatch %}
    {% endfor %}
};