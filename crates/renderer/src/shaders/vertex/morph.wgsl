//***** MORPHS *****
const MAX_MORPH_TARGETS: u32 = 8;

// alignment rules dictate we can't just have an array of floats 
@group(2) @binding(0)
var<uniform> morph_weights: array<vec4<f32>, MAX_MORPH_TARGETS/ 4u>; 

// this is the array of morph target deltas
// always interleaved as position, normal, tangent
// so we can use the same array for all three
// even as we index sequentially
@group(3) @binding(0)
var<storage, read> morph_values: array<MorphValues>; 

struct MorphValues {
    position: vec3<f32>,
    normal: vec3<f32>,
    tangent: vec3<f32>,
};

fn apply_morphs(input: VertexInput) -> VertexInput {
    var output = input;

    for (var i = 0u; i < MAX_MORPH_TARGETS; i = i + 1u) {
        // 2d index into the array
        // 4 floats per vec4, so we need to divide by 4
        // and then mod by 4 to get the index into the vec4
        let morph_weight = morph_weights[i / 4u][i % 4u];

        let morph_values = morph_values[i];

        output.position += morph_weight * morph_values.position; 
        // #IF normals
        output.normal += morph_weight * morph_values.normal; 
        // #IF tangents 
        output.tangent += morph_weight * morph_values.tangent;
    }

    return output;
}