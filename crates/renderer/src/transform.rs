use slotmap::{new_key_type, SlotMap};

#[derive(Default)]
pub struct Transforms {
    // TODO - replace with slotmap
    local_lookup: SlotMap<TransformKey, Transform>,
    world_lookup: SlotMap<TransformKey, glam::Mat4>,
    child_lookup: Vec<TransformKey>,
    parent_lookup: Vec<TransformKey>,
}

// TODO - replace with slotmap
new_key_type! {
    pub struct TransformKey;
}

// TODO - translation, rotation, scale, origin?
pub struct Transform {
    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
    pub origin: glam::Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            translation: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
            origin: glam::Vec3::ZERO,
        }
    }

}