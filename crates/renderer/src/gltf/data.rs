use awsm_renderer_core::image::ImageData;

use super::{buffers::GltfBuffers, error::Result, loader::GltfLoader};

#[derive(Debug)]
pub struct GltfData {
    pub doc: gltf::Document,
    pub buffers: GltfBuffers,
    pub images: Vec<ImageData>,
}

impl GltfLoader {
    pub fn into_data(self) -> Result<GltfData> {
        let buffers = GltfBuffers::new(&self.doc, self.buffers)?;

        Ok(GltfData {
            doc: self.doc,
            images: self.images,
            buffers,
        })
    }
}
