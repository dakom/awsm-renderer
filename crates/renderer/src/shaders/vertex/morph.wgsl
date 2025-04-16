//***** MORPHS *****
const MAX_MORPH_WEIGHTS:u32= 8;

// alignment rules dictate we can't just have an array of floats 
@group(2) @binding(0)
var<uniform> morph_weights: array<vec4<f32>, MAX_MORPH_WEIGHTS/ 4u>; 

// this is the array of morph target deltas
// always interleaved as position, normal, tangent
// so we can use the same array for all three
// even as we index sequentially
@group(3) @binding(0)
var<storage, read> morph_values: array<f32>; 

// This changes per-shader via constant overrides
// the rest of the calculations flow from the presence of attributes (which cause the shader to change anyway)
// i.e. if a shader supports normals, then even if there are no morphs for it, 
// calculations will be done and the buffer data has zeroes filled in for the morphs
@id(1) override MAX_MORPH_TARGETS:u32;

fn apply_morphs(input: VertexInput) -> VertexInput {
    var output = input;

    // target_size is the total number of floats for each morph_target (for a given vertex, not across all of them) 
    var target_size = 3u; // vec3 for position
    // #IF normals
    target_size += 3u; // vec3 for normals
    // #IF tangents
    target_size += 3u; // vec3 for tangents

    // all_targets_size is the total number of floats for all morph_targets (for a given vertex, not across all of them)
    let all_targets_size = target_size * MAX_MORPH_TARGETS; 

    for (var morph_target = 0u; morph_target < MAX_MORPH_TARGETS; morph_target = morph_target + 1u) {
        // 2d index into the array
        // 4 floats per vec4, so we need to divide by 4 to get "which vec4" we are in
        // and then mod by 4 to get the index into that vec4
        var morph_weight = morph_weights[morph_target / 4u][morph_target % 4u];

        // For each vertex, skip the "full" morph-target data for all targets
        // then, for reach morph target, skip the morph-target data up until this count
        var offset = (input.vertex_index * all_targets_size) + (morph_target * target_size);

        let morph_position = vec3<f32>(morph_values[offset], morph_values[offset + 1u], morph_values[offset + 2u]);
        output.position += morph_weight * morph_position; 

        // #SECTIONIF normals
        offset += 3;
        let morph_normal = vec3<f32>(morph_values[offset], morph_values[offset + 1u], morph_values[offset + 2u]);
        output.normal += morph_weight * morph_normal; 
        // #ENDIF

        // #SECTIONIF tangents
        offset += 3;
        let morph_tangent = vec3<f32>(morph_values[offset], morph_values[offset + 1u], morph_values[offset + 2u]);
        output.tangent += morph_weight * morph_tangent; 
        // #ENDIF

    }

    return output;
}