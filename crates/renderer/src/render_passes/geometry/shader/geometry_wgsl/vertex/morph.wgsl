//***** MORPHS *****

// The morph weights
// but the first value is actually the morph_target count
@group(2) @binding(0)
var<storage, read> morph_weights: array<f32>;

// this is the array of morph target deltas
// always interleaved as position, normal, tangent
// so we can use the same array for all three
// even as we index sequentially
@group(2) @binding(1)
var<storage, read> morph_values: array<f32>; 

fn apply_morphs(input: VertexInput) -> VertexInput {
    var output = input;

    let morph_target_len = u32(morph_weights[0]); // the first value is the morph_target count

    // target_size is the total number of floats for each morph_target (for a given vertex, not across all of them) 
    var target_size = 0u; // vec3 for position

    {% if morphs.position %}
        target_size += 3u; // vec3 for normals
    {% endif %}

    {% if morphs.normal %}
        target_size += 3u; // vec3 for normals
    {% endif %}

    {% if morphs.tangent %}
    // vec3 for tangents, not vec4
    // from spec: "Note that the W component for handedness is omitted when targeting TANGENT data since handedness cannot be displaced."
        target_size += 3u; 
    {% endif %}

    // TODO - TEXCOORD_n and COLOR_n
    // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#morph-targets

    // all_targets_size is the total number of floats for all morph_targets (for a given vertex, not across all of them)
    let all_targets_size = target_size * morph_target_len; 

    for (var morph_target = 0u; morph_target < morph_target_len; morph_target = morph_target + 1u) {
        // 2d index into the array
        // 4 floats per vec4, so we need to divide by 4 to get "which vec4" we are in
        // and then mod by 4 to get the index into that vec4
        var morph_weight = morph_weights[morph_target + 1u]; // the first value is the morph_target count so we skip it

        // For each vertex, skip the "full" morph-target data for all targets
        // then, for reach morph target, skip the morph-target data up until this count
        var offset = (input.vertex_index * all_targets_size) + (morph_target * target_size);

        {% if morphs.position %}
            let morph_position = vec3<f32>(morph_values[offset], morph_values[offset + 1u], morph_values[offset + 2u]);
            output.position += morph_weight * morph_position; 
            offset += 3;
        {% endif %}

        {% if morphs.normal %}
            let morph_normal = vec3<f32>(morph_values[offset], morph_values[offset + 1u], morph_values[offset + 2u]);
            output.normal += morph_weight * morph_normal; 
            offset += 3;
        {% endif %}

        {% if morphs.tangent %}
            let morph_tangent = vec3<f32>(morph_values[offset], morph_values[offset + 1u], morph_values[offset + 2u]);
            // vec3 for tangents, not vec4
            // from spec: "Note that the W component for handedness is omitted when targeting TANGENT data since handedness cannot be displaced."
            output.tangent.x += morph_weight * morph_tangent.x; 
            output.tangent.y += morph_weight * morph_tangent.y; 
            output.tangent.z += morph_weight * morph_tangent.z; 
            offset += 3;
        {% endif %}

    }

    return output;
}