use awsm_renderer_core::image::ImageData;

use super::{buffers::GltfBuffers, error::Result, loader::GltfLoader};

pub struct GltfData {
    pub doc: gltf::Document,
    pub buffers: GltfBuffers,
    pub images: Vec<ImageData>,
    pub hints: GltfDataHints,
}

impl GltfData {
    pub fn heavy_clone(&self) -> Self {
        Self {
            doc: self.doc.clone(),
            buffers: self.buffers.heavy_clone(),
            images: self.images.clone(),
            hints: self.hints.clone(),
        }
    }
}

#[derive(Default, Clone)]
pub struct GltfDataHints {
    pub hud: bool,
    pub hidden: bool,
}

impl GltfDataHints {
    pub fn with_hud(mut self, hud: bool) -> Self {
        self.hud = hud;
        self
    }

    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }
}

impl GltfLoader {
    pub fn into_data(self, hints: Option<GltfDataHints>) -> Result<GltfData> {
        let hints = hints.unwrap_or_default();
        let buffers = GltfBuffers::new(&self.doc, self.buffers, hints.clone())?;

        Ok(GltfData {
            doc: self.doc,
            images: self.images,
            buffers,
            hints,
        })
    }
}
