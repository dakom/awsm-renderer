{% if geometry.as_mesh().morphs.any() %}
    {% include "vertex/mesh/morph.wgsl" %}
{% endif %}

{% if geometry.as_mesh().skins > 0 %}
    {% include "vertex/mesh/skin.wgsl" %}
{% endif %}

@group(1) @binding(0)
var<uniform> u_transform: TransformUniform;

struct TransformUniform {
    model: mat4x4<f32>,
};


//***** INPUT/OUTPUT *****
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    {% for loc in geometry.as_mesh().vertex_input_locations %}
        {%- match loc.interpolation %}
            {% when Some with (interpolation) %}
                @location({{ loc.location }}) @interpolate({{ interpolation }}) {{ loc.name }}: {{ loc.data_type }},
            {% when _ %}
                @location({{ loc.location }}) {{ loc.name }}: {{ loc.data_type }},
        {% endmatch %}
    {% endfor %}
};

//***** MAIN *****
@vertex
fn vert_main(raw_input: VertexInput) -> FragmentInput {
    var input = raw_input;

    // morphs first: https://github.com/KhronosGroup/glTF/issues/1646#issuecomment-542815692
    {% if geometry.as_mesh().morphs.any() %}
    input = apply_morphs(input);
    {% endif %}

    {% if geometry.as_mesh().skins > 0 %}
    input = apply_skin(input);
    {% endif %}

    // Transform the vertex position by the model matrix, and then by the view projection matrix
    {% if geometry.as_mesh().has_instance_transforms %}
        // Transform the vertex position by the instance transform
        let instance_transform = mat4x4<f32>(
            raw_input.instance_transform_row_0,
            raw_input.instance_transform_row_1,
            raw_input.instance_transform_row_2,
            raw_input.instance_transform_row_3,
        );

        let model_transform = u_transform.model * instance_transform;
    {% else %}
        let model_transform = u_transform.model;
    {% endif %}

    var output: FragmentInput;

    var pos = model_transform * vec4<f32>(input.position, 1.0);
    output.world_position = pos.xyz;
    {% if geometry.as_mesh().has_normals %}
        output.world_normal = normalize((model_transform * vec4<f32>(input.normal, 0.0)).xyz);
    {% endif %}
    output.clip_position = camera.view_proj * pos;

    {% for assignment in material.as_pbr().vertex_to_fragment_assignments %}
        output.{{ assignment.fragment_name }} = input.{{ assignment.vertex_name }};
    {% endfor %}

    return output;
}