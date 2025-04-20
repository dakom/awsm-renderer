use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};

use crate::{buffers::storage::StorageBufferKey, transform::TransformKey, AwsmRenderer};

use super::{data::GltfData, error::AwsmGltfError};

mod animation;
mod mesh;
mod skin;
mod transforms;

pub(super) struct GltfPopulateContext {
    pub data: Arc<GltfData>,
    pub morph_buffer_storage_key: Option<StorageBufferKey>,
    pub node_to_transform: Mutex<HashMap<usize, TransformKey>>,
    pub transform_is_joint: Mutex<HashSet<TransformKey>>,
}

impl AwsmRenderer {
    pub async fn populate_gltf(
        &mut self,
        gltf_data: GltfData,
        scene: Option<usize>,
    ) -> anyhow::Result<()> {
        let gltf_data = Arc::new(gltf_data);
        self.gltf.raw_datas.push(gltf_data.clone());

        // morph weights are all populated in GltfData before we get here
        // (similar to vertex and index buffers)
        let morph_buffer_storage_key = if let Some(morph_buffer) = &gltf_data.buffers.morph_buffer {
            Some(self.storage.insert(morph_buffer.clone()))
        } else {
            None
        };

        let ctx = GltfPopulateContext {
            data: gltf_data,
            morph_buffer_storage_key,
            node_to_transform: Mutex::new(HashMap::new()),
            transform_is_joint: Mutex::new(HashSet::new()),
        };

        let scene = match scene {
            Some(index) => ctx
                .data
                .doc
                .scenes()
                .nth(index)
                .ok_or(AwsmGltfError::InvalidScene(index))?,
            None => ctx
                .data
                .doc
                .default_scene()
                .ok_or(AwsmGltfError::NoDefaultScene)?,
        };

        for node in scene.nodes() {
            self.populate_gltf_node_transform(&ctx, &node, None)?;
        }

        for node in scene.nodes() {
            self.populate_gltf_node_skin(&ctx, &node)?;
        }

        for node in scene.nodes() {
            self.populate_gltf_node_animation(&ctx, &node)?;
        }

        for node in scene.nodes() {
            self.populate_gltf_node_mesh(&ctx, &node).await?;
        }

        Ok(())
    }
}
