// Modal matrix

fn get_model_transform(byte_offset: u32) -> mat4x4<f32> {
    return model_transforms[byte_offset / 64];
}
