use awsm_renderer_core::renderer::AwsmRendererWebGpu;

use super::{
    buffers::GltfBuffers,
    error::{AwsmGltfError, Result},
    loader::GltfLoader,
};

#[derive(Debug)]
pub struct GltfData {
    pub doc: gltf::Document,
    pub buffers: GltfBuffers,
    pub textures: Vec<web_sys::GpuTexture>,
}

impl GltfLoader {
    pub async fn into_data(self, gpu: &AwsmRendererWebGpu) -> Result<GltfData> {
        let buffers = GltfBuffers::new(&self.doc, self.buffers)?;

        let mut textures = Vec::with_capacity(self.images.len());

        for image in self.images {
            // TODO: generate mipmaps, maybe depending on filter settings
            // "from spec: To properly support mipmap modes, client implementations SHOULD generate mipmaps at runtime."
            let texture = image
                .create_texture(gpu, None, false)
                .map_err(AwsmGltfError::CreateTexture)?;
            textures.push(texture);
        }

        Ok(GltfData {
            doc: self.doc,
            textures,
            buffers,
        })
    }
}
