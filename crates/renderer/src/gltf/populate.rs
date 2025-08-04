use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use awsm_renderer_core::{image::atlas::ImageAtlas, renderer::AwsmRendererWebGpu};

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
mod transforms;

pub(crate) struct GltfPopulateContext {
    pub data: Arc<GltfData>,
    pub image_atlas: Mutex<GltfImageAtlas>,
    pub node_to_transform: Mutex<HashMap<GltfIndex, TransformKey>>,
    pub node_to_skin: Mutex<HashMap<GltfIndex, SkinKey>>,
    pub transform_is_joint: Mutex<HashSet<TransformKey>>,
    pub transform_is_instanced: Mutex<HashSet<TransformKey>>,
    pub generate_mipmaps: bool,
}

type GltfIndex = usize;
type LayerIndex = usize;
type EntryIndex = usize;
pub struct GltfImageAtlas {
    pub atlas: ImageAtlas,
    pub lookup: HashMap<GltfIndex, (LayerIndex, EntryIndex)>,
    pub counter: u64,
}

impl GltfImageAtlas {
    pub async fn new(gpu: &AwsmRendererWebGpu) -> Self {
        let max_dimension_2d = gpu.device.limits().max_texture_dimension_2d();
        let max_depth_2d_array = gpu.device.limits().max_texture_array_layers();

        tracing::info!(
            "Creating GLTF image atlas with max texture dimension 2D: {}, max depth 2D array: {}",
            max_dimension_2d,
            max_depth_2d_array
        );

        Self {
            atlas: ImageAtlas::new(max_dimension_2d, max_dimension_2d, 8),
            lookup: HashMap::new(),
            counter: 0,
        }
    }
}

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
            image_atlas: Mutex::new(GltfImageAtlas::new(&self.gpu).await),
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

        {
            // let atlas = ctx.image_atlas.lock().unwrap();
            // tracing::info!("Image Atlas populated with {} layers and {} entries",
            //     atlas.atlas.layers.len(),
            //     atlas.atlas.layers.iter().map(|l| l.entries.len()).sum::<usize>()
            // );
            // for (layer_index, layer) in atlas.atlas.layers.iter().enumerate() {
            //     for (entry_index, entry) in layer.entries.iter().enumerate() {
            //         tracing::info!(
            //             "Image Atlas Entry: Layer: {}, Entry: {}, Pixel Offset: ({}, {}), size: ({}, {})",
            //             layer_index, entry_index, entry.pixel_offset.0, entry.pixel_offset.1, entry.image_data.size().0, entry.image_data.size().1
            //         );
            //     }
            // }
        }

        let atlas = ctx.image_atlas.into_inner().unwrap();
        atlas.atlas.write_texture_array(&self.gpu, None).await?;

        Ok(())
    }
}
