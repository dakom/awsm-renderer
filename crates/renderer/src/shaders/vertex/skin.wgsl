//***** SKINS *****
// probably need to adjust this for > 1 sets
@group(2) @binding(2)
var<storage, read> skin_joint_values: array<f32>; 

fn apply_skin(input: VertexInput) -> VertexInput {
    var output = input;

    let values_mat = mat4x4f(
        skin_joint_values[0], skin_joint_values[1], skin_joint_values[2], skin_joint_values[3],
        skin_joint_values[4], skin_joint_values[5], skin_joint_values[6], skin_joint_values[7],
        skin_joint_values[8], skin_joint_values[9], skin_joint_values[10], skin_joint_values[11],
        skin_joint_values[12], skin_joint_values[13], skin_joint_values[14], skin_joint_values[15]
    );

    let skin_mat = mat4x4f(
        input.joint_weights[0] * values_mat[input.joint_indices[0]],
        input.joint_weights[1] * values_mat[input.joint_indices[1]],
        input.joint_weights[2] * values_mat[input.joint_indices[2]],
        input.joint_weights[3] * values_mat[input.joint_indices[3]]
    );

    output.position = (skin_mat * vec4<f32>(input.position, 1.0)).xyz;

    return output;
}