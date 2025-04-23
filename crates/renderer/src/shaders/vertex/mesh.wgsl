
@group(1) @binding(0)
var<uniform> u_transform: TransformUniform;

struct TransformUniform {
    model: mat4x4<f32>,
};


//***** INPUT/OUTPUT *****
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    {% for loc in vertex_input_locations %}
        {%- match loc.interpolation %}
            {% when Some with (interpolation) %}
                @location({{ loc.location }}) @interpolate({{ interpolation }}) {{ loc.name }}: {{ loc.data_type }},
            {% when _ %}
                @location({{ loc.location }}) {{ loc.name }}: {{ loc.data_type }},
        {% endmatch %}
    {% endfor %}
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

//***** MAIN *****
@vertex
fn vert_main(raw_input: VertexInput) -> VertexOutput {
    var input = raw_input;


    {% if has_morphs %}
    input = apply_morphs(input);
    {% endif %}

    {% if has_skins %}
    input = apply_skin(input);
    {% endif %}

    // Transform the vertex position by the model matrix, and then by the view projection matrix
    var pos = u_transform.model * vec4<f32>(input.position, 1.0);
    pos = camera.view_proj * pos;

    // Assign and return final output
    let output = VertexOutput(pos);

    return output;
}