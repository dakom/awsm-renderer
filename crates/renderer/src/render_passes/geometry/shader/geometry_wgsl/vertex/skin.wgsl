//***** SKINS *****
@group(3) @binding(2)
var<storage, read> skin_joint_matrices: array<mat4x4<f32>>; 

// Joint buffer - exploded per vertex (matches morph pattern)
// However, to stay under bind group limits, we interleave indices with weights
// and get our index back losslessly via bitcast
// Layout: exploded vertex 0: [joints_0, joints_1, ...], exploded vertex 1: [joints_0, joints_1, ...], etc.
@group(3) @binding(3)
var<storage, read> skin_joint_index_weights: array<f32>;

// Each skin set has 8 f32 values (4 interleaved index/weight pairs)
const floats_per_set = 8u;

/// Applies skeletal skinning with support for multiple skin sets per vertex
fn apply_position_skin(input: VertexInput) -> VertexInput {
    var output = input;

    let skin_sets_count = mesh_meta.skin_sets_len;

    let original_position = vec4<f32>(input.position, 1.0);

    // Calculate base offset for this exploded vertex's skin data
    let base_offset = input.vertex_index * skin_sets_count * floats_per_set;

    var skin_matrix: mat4x4<f32>;
    
    // UNROLLED SKIN SETS
    
    {% for i in 0..max_skin_unroll %}
        if skin_sets_count >= {{ i + 1}}u {
            let buffer_offset = base_offset + ({{ i }}u * floats_per_set);

            let joint_index_0 = bitcast<u32>(skin_joint_index_weights[buffer_offset]);
            let joint_weight_0 = skin_joint_index_weights[buffer_offset + 1u];

            let joint_index_1 = bitcast<u32>(skin_joint_index_weights[buffer_offset + 2u]);
            let joint_weight_1 = skin_joint_index_weights[buffer_offset + 3u];

            let joint_index_2 = bitcast<u32>(skin_joint_index_weights[buffer_offset + 4u]);
            let joint_weight_2 = skin_joint_index_weights[buffer_offset + 5u];

            let joint_index_3 = bitcast<u32>(skin_joint_index_weights[buffer_offset + 6u]);
            let joint_weight_3 = skin_joint_index_weights[buffer_offset + 7u];

            let skin_mat_acc = joint_weight_0 * skin_joint_matrices[joint_index_0]
                            + joint_weight_1 * skin_joint_matrices[joint_index_1]
                            + joint_weight_2 * skin_joint_matrices[joint_index_2]
                            + joint_weight_3 * skin_joint_matrices[joint_index_3];

            {% if i == 0 %}
                skin_matrix = skin_mat_acc;
            {% else %}
                skin_matrix = skin_matrix * skin_mat_acc;
            {% endif %}
        }
    {% endfor %}

    // LOOP FOR REMAINING SKIN SETS
    if skin_sets_count > {{ max_skin_unroll }}u {
        for (var skin_set_index = {{ max_skin_unroll }}u; skin_set_index < skin_sets_count; skin_set_index = skin_set_index + 1u) {
            let buffer_offset = base_offset + (skin_set_index * floats_per_set);

            let joint_index_0 = bitcast<u32>(skin_joint_index_weights[buffer_offset]);
            let joint_weight_0 = skin_joint_index_weights[buffer_offset + 1u];

            let joint_index_1 = bitcast<u32>(skin_joint_index_weights[buffer_offset + 2u]);
            let joint_weight_1 = skin_joint_index_weights[buffer_offset + 3u];

            let joint_index_2 = bitcast<u32>(skin_joint_index_weights[buffer_offset + 4u]);
            let joint_weight_2 = skin_joint_index_weights[buffer_offset + 5u];

            let joint_index_3 = bitcast<u32>(skin_joint_index_weights[buffer_offset + 6u]);
            let joint_weight_3 = skin_joint_index_weights[buffer_offset + 7u];

            let skin_mat_acc = joint_weight_0 * skin_joint_matrices[joint_index_0]
                             + joint_weight_1 * skin_joint_matrices[joint_index_1]
                             + joint_weight_2 * skin_joint_matrices[joint_index_2]
                             + joint_weight_3 * skin_joint_matrices[joint_index_3];

            skin_matrix = skin_matrix * skin_mat_acc;
        }
    }

    output.position = (skin_matrix * original_position).xyz;

    return output;
}
