use awsm_renderer_core::image::ImageData;

use super::{buffers::GltfBuffers, error::Result, loader::GltfLoader};

pub struct GltfData {
    pub doc: gltf::Document,
    pub buffers: GltfBuffers,
    pub images: Vec<ImageData>,
}

impl GltfData {
    pub fn heavy_clone(&self) -> Self {
        Self {
            doc: self.doc.clone(),
            buffers: self.buffers.heavy_clone(),
            images: self.images.clone(),
        }
    }
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
