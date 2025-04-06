use crate::AwsmRenderer;
use awsm_renderer_core::image::ImageLoader;

use super::{buffers::GltfBuffers, error::Result, loader::GltfLoader};

#[derive(Debug)]
pub struct GltfData {
    pub doc: gltf::Document,
    pub buffers: GltfBuffers,
    // TODO - create textures instead?
    pub images: Vec<ImageLoader>,
}

impl GltfData {
    pub async fn new(renderer: &AwsmRenderer, loader: GltfLoader) -> Result<Self> {
        let buffers = GltfBuffers::new(renderer, &loader.doc, loader.buffers).await?;

        Ok(Self {
            doc: loader.doc,
            images: loader.images,
            buffers,
        })
    }
}
