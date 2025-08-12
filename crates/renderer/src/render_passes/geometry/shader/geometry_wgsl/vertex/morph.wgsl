//***** MORPHS *****

// The morph weights
@group(2) @binding(0)
var<storage, read> geometry_morph_weights: array<f32>;

// this is the array of morph target deltas
// always interleaved as position, normal, tangent
// so we can use the same array for all three
// even as we index sequentially
@group(2) @binding(1)
var<storage, read> geometry_morph_values: array<f32>; 

fn apply_position_morphs(input: VertexInput) -> VertexInput {
    var output = input;

    let morph_targets_len = mesh_meta.morph_geometry_target_len;

    let morph_target_count = 3u; // just position, vec3

    // all_targets_count is the total number of floats for all morph_targets (for a given vertex, not across all of them)
    let all_targets_count = morph_targets_len * morph_target_count; 

    for (var morph_target = 0u; morph_target < morph_targets_len; morph_target = morph_target + 1u) {
        // 2d index into the array
        // 4 floats per vec4, so we need to divide by 4 to get "which vec4" we are in
        // and then mod by 4 to get the index into that vec4
        var morph_weight = geometry_morph_weights[morph_target]; // the first value is the morph_target count so we skip it

        // For each vertex, skip the "full" morph-target data for all targets
        // then, for reach morph target, skip the morph-target data up until this count
        var offset = (input.vertex_index * all_targets_count) + (morph_target * morph_target_count);

        let morph_position = vec3<f32>(geometry_morph_values[offset], geometry_morph_values[offset + 1u], geometry_morph_values[offset + 2u]);
        output.position += morph_weight * morph_position; 
    }

    return output;
}