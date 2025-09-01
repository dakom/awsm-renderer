// Modal matrix
@group(1) @binding(0)
var<storage, read> model_transforms : array<mat4x4<f32>>;

fn get_model_transform(byte_offset: u32) -> mat4x4<f32> {
    return model_transforms[byte_offset / 64];
}