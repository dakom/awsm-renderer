use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, DenseSlotMap};

use crate::dynamic_uniform_buffer::DynamicUniformBuffer;

use super::error::{AwsmMeshError, Result};
use super::Mesh;

const MORPH_WEIGHTS_BYTE_SIZE: usize = 32; // 8xf32 is 32 bytes

pub struct Meshes {
    list: DenseSlotMap<MeshKey, Mesh>,
    morph_weights_buffer: DynamicUniformBuffer<MeshKey, MORPH_WEIGHTS_BYTE_SIZE>,
}

impl Meshes {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            list: DenseSlotMap::with_key(),
            morph_weights_buffer: DynamicUniformBuffer::new(gpu, Some("MorphWeights".to_string()))
                .unwrap(),
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Mesh> {
        self.list.values()
    }

    pub fn insert(&mut self, mesh: Mesh) -> MeshKey {
        self.list.insert(mesh)
    }

    pub fn get_mut(&mut self, mesh_key: MeshKey) -> Result<&mut Mesh> {
        self.list
            .get_mut(mesh_key)
            .ok_or(AwsmMeshError::MeshNotFound(mesh_key))
    }
}

new_key_type! {
    pub struct MeshKey;
}
