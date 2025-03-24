#[derive(Default)]
pub struct Transforms {
    // TODO - replace with slotmap
    local_lookup: Vec<Transform>,
    world_lookup: Vec<Transform>,
    child_lookup: Vec<TransformKey>,
    parent_lookup: Vec<TransformKey>,
}

// TODO - replace with slotmap
pub type TransformKey = usize;

// TODO - translation, rotation, scale, origin?
pub struct Transform {
}

impl Transform {
    pub fn new() -> Self {
        Self {
        }
    }

}