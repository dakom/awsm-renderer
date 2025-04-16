use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, DenseSlotMap};

use super::error::{AwsmMeshError, Result};
use super::Mesh;
use super::morphs::Morphs;


pub struct Meshes {
    list: DenseSlotMap<MeshKey, Mesh>,
    pub morphs: Morphs,
}

impl Meshes {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Self {
            list: DenseSlotMap::with_key(),
            morphs: Morphs::new(gpu)?,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (MeshKey, &Mesh)> {
        self.list.iter()
    }

    pub fn insert(&mut self, mesh: Mesh) -> MeshKey {
        self.list.insert(mesh)
    }

    pub fn get_mut(&mut self, mesh_key: MeshKey) -> Result<&mut Mesh> {
        self.list
            .get_mut(mesh_key)
            .ok_or(AwsmMeshError::MeshNotFound(mesh_key))
    }

    pub fn remove(&mut self, mesh_key: MeshKey) -> Option<Mesh> {
        if let Some(mesh) = self.list.remove(mesh_key) {
            if let Some(morph_key) = mesh.morph_key {
                self.morphs.remove(morph_key);
            }
            Some(mesh)
        } else {
            None
        }
    }


    pub fn write_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        self.morphs.write_weights_gpu(gpu)
    }

}

new_key_type! {
    pub struct MeshKey;
}
