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