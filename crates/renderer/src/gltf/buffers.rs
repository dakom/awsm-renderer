use super::cache::GltfResourceKey;

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct BufferKey {
    pub gltf_res_key: GltfResourceKey,
    pub index: usize
}