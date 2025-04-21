use awsm_renderer_core::image::ImageLoader;

use super::{
    buffers::GltfBuffers,
    error::{AwsmGltfError, Result},
    loader::GltfLoader,
};

#[derive(Debug)]
pub struct GltfData {
    pub doc: gltf::Document,
    pub buffers: GltfBuffers,
    // TODO - create textures instead?
    pub images: Vec<ImageLoader>,
}

impl TryFrom<GltfLoader> for GltfData {
    type Error = AwsmGltfError;

    fn try_from(loader: GltfLoader) -> Result<Self> {
        let buffers = GltfBuffers::new(&loader.doc, loader.buffers)?;

        Ok(Self {
            doc: loader.doc,
            images: loader.images,
            buffers,
        })
    }
}
