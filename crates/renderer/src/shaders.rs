#[repr(u32)]
pub(crate) enum BindGroup {
    Camera = 0,
    Transform = 1,
}

#[repr(u32)]
pub(crate) enum BindGroupBinding {
    Camera = 0,
}
