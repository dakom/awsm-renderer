
@group(1) @binding(0)
var<uniform> u_transform: TransformUniform;

struct TransformUniform {
    model: mat4x4<f32>,
};


//***** INPUT/OUTPUT *****
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,

    @location(0) position: vec3<f32>,

    {% if has_normal %}
    @location(1) normal: vec3<f32>,
    {% endif %}

    {% if has_tangent %}
    @location(2) tangent: vec3<f32>,
    {% endif %}

    {% if skin_joint_sets > 0 %}
        @location(3) @interpolate(flat) joint_indices_1: vec4<u32>,
        @location(4) joint_weights_1: vec4<f32>,
    {% endif %}
    {% if skin_joint_sets > 1 %}
        @location(5) @interpolate(flat) joint_indices_2: vec4<u32>,
        @location(6) joint_weights_2: vec4<f32>,
    {% endif %}
    {% if skin_joint_sets > 2 %}
        @location(7) @interpolate(flat) joint_indices_3: vec4<u32>,
        @location(8) joint_weights_3: vec4<f32>,
    {% endif %}
    {% if skin_joint_sets > 3 %}
        More than 3 joint sets not supported in vertex shader
    {% endif %}
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

    {% if skin_joint_sets > 0 %}
    input = apply_skin(input);
    {% endif %}

    // Transform the vertex position by the model matrix, and then by the view projection matrix
    var pos = u_transform.model * vec4<f32>(input.position, 1.0);
    pos = camera.view_proj * pos;

    // Assign and return final output
    let output = VertexOutput(pos);

    return output;
}