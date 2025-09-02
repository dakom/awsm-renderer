//***** MORPHS *****

// The morph weights
@group(3) @binding(0)
var<storage, read> geometry_morph_weights: array<f32>;

// this is the array of morph target deltas
// always interleaved as position, normal, tangent
// so we can use the same array for all three
// even as we index sequentially
@group(3) @binding(1)
var<storage, read> geometry_morph_values: array<f32>; 

fn apply_position_morphs(input: VertexInput) -> VertexInput {
    var output = input;

    let target_count = mesh_meta.morph_geometry_target_len;

    // Each target contributes 3 floats (vec3 position delta)
    let floats_per_target = 3u;
    let total_floats_per_vertex = target_count * floats_per_target;
    
    // Calculate base offset for this exploded vertex's morph data
    let base_weights_offset = mesh_meta.morph_geometry_weights_offset / 4;
    let base_values_offset = (mesh_meta.morph_geometry_values_offset / 4) +  input.vertex_index * total_floats_per_vertex;

    // UNROLLED TARGETS for better performance
    {% for i in 0..max_morph_unroll %}
        if target_count >= {{ i+1 }}u {
            let weight_offset = base_weights_offset + {{ i }}; 
            let weight = geometry_morph_weights[weight_offset];
            let value_offset = base_values_offset + ({{ i }} * floats_per_target);
            let morph_delta = vec3<f32>(
                geometry_morph_values[value_offset],
                geometry_morph_values[value_offset + 1u], 
                geometry_morph_values[value_offset + 2u]
            );
            output.position += weight * morph_delta;
        }
    {% endfor %}

    // LOOP FOR REMAINING TARGETS 
    if target_count > {{ max_morph_unroll }}u {
        for (var target_index = {{ max_morph_unroll }}u; target_index < target_count; target_index = target_index + 1u) {
            let weight_offset = base_weights_offset + target_index; 
            let weight = geometry_morph_weights[weight_offset];
            let value_offset = base_values_offset + (target_index * floats_per_target);
            let morph_delta = vec3<f32>(
                geometry_morph_values[value_offset],
                geometry_morph_values[value_offset + 1u], 
                geometry_morph_values[value_offset + 2u]
            );
            
            output.position += weight * morph_delta;
        }
    }

    return output;
}
