//***** SKINS *****
@group(2) @binding(2)
var<storage, read> skin_joint_mat: array<mat4x4f>; 

fn apply_skin(input: VertexInput) -> VertexInput {
    var output = input;

    var skin_mat = input.joint_weights_1[0] * skin_joint_mat[input.joint_indices_1[0]]
        + input.joint_weights_1[1] * skin_joint_mat[input.joint_indices_1[1]]
        + input.joint_weights_1[2] * skin_joint_mat[input.joint_indices_1[2]]
        + input.joint_weights_1[3] * skin_joint_mat[input.joint_indices_1[3]];

    {% if skin_joint_sets > 1 %}
    skin_mat += input.joint_weights_2[0] * skin_joint_mat[input.joint_indices_2[0]]
        + input.joint_weights_2[1] * skin_joint_mat[input.joint_indices_2[1]]
        + input.joint_weights_2[2] * skin_joint_mat[input.joint_indices_2[2]]
        + input.joint_weights_2[3] * skin_joint_mat[input.joint_indices_2[3]];
    {% endif %}

    {% if skin_joint_sets > 2 %}
    skin_mat += input.joint_weights_3[0] * skin_joint_mat[input.joint_indices_3[0]]
        + input.joint_weights_3[1] * skin_joint_mat[input.joint_indices_3[1]]
        + input.joint_weights_3[2] * skin_joint_mat[input.joint_indices_3[2]]
        + input.joint_weights_3[3] * skin_joint_mat[input.joint_indices_3[3]];
    {% endif %}

    {% if skin_joint_sets > 3 %}
        More than 3 joint sets not supported in vertex shader
    {% endif %}

    output.position = (skin_mat * vec4<f32>(input.position, 1.0)).xyz;

    return output;
}