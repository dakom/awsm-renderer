//***** SKINS *****
@group(3) @binding(2)
var<storage, read> skin_joint_mat: array<mat4x4f>; 

fn apply_skin(input: VertexInput) -> VertexInput {
    var output = input;

    {% for i in 0..skins %}
        let skin_mat_acc = input.skin_weight_{{ i }}[0] * skin_joint_mat[input.skin_joint_{{ i }}[0]]
            + input.skin_weight_{{ i }}[1] * skin_joint_mat[input.skin_joint_{{ i }}[1]]
            + input.skin_weight_{{ i }}[2] * skin_joint_mat[input.skin_joint_{{ i }}[2]]
            + input.skin_weight_{{ i }}[3] * skin_joint_mat[input.skin_joint_{{ i }}[3]];

        {% if i == 0 %}
            var skin_mat = skin_mat_acc;
        {% else %}
            skin_mat = skin_mat * skin_mat_acc;
        {% endif %}
    {% endfor %}

    output.position = (skin_mat * vec4<f32>(input.position, 1.0)).xyz;

    return output;
}