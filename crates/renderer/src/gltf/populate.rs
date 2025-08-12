use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use awsm_renderer_core::{renderer::AwsmRendererWebGpu, texture::mega_texture::MegaTexture};

use crate::{
    mesh::skins::SkinKey,
    textures::{SamplerKey, TextureKey},
    transforms::TransformKey,
    AwsmRenderer,
};

use super::{data::GltfData, error::AwsmGltfError};

mod animation;
mod extensions;
mod material;
mod mesh;
mod skin;
pub(super) mod transforms;

pub(crate) struct GltfPopulateContext {
    pub data: Arc<GltfData>,
    pub textures: Mutex<HashMap<GltfIndex, TextureKey>>,
    pub node_to_transform: Mutex<HashMap<GltfIndex, TransformKey>>,
    pub node_to_skin: Mutex<HashMap<GltfIndex, SkinKey>>,
    pub transform_is_joint: Mutex<HashSet<TransformKey>>,
    pub transform_is_instanced: Mutex<HashSet<TransformKey>>,
    pub generate_mipmaps: bool,
}

type GltfIndex = usize;

impl AwsmRenderer {
    pub async fn populate_gltf(
        &mut self,
        gltf_data: GltfData,
        scene: Option<usize>,
        generate_mipmaps: bool,
    ) -> anyhow::Result<()> {
        #[allow(clippy::arc_with_non_send_sync)]
        let gltf_data = Arc::new(gltf_data);
        self.gltf.raw_datas.push(gltf_data.clone());

        let ctx = GltfPopulateContext {
            data: gltf_data,
            textures: Mutex::new(HashMap::new()),
            node_to_transform: Mutex::new(HashMap::new()),
            node_to_skin: Mutex::new(HashMap::new()),
            transform_is_joint: Mutex::new(HashSet::new()),
            transform_is_instanced: Mutex::new(HashSet::new()),
            generate_mipmaps,
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
            self.populate_gltf_node_extension_instancing(&ctx, &node)?;
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

        self.finalize_gpu_textures().await?;

        Ok(())
    }
}
