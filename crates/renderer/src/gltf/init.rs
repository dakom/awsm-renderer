use crate::AwsmRenderer;
use anyhow::Result;

use super::loader::GltfResource;

impl AwsmRenderer {
    pub async fn init_gltf(&mut self, gltf_res: &GltfResource) -> Result<()> {
        // todo - port from `populate.rs`
        tracing::info!("initializing {:#?}", gltf_res.gltf);
        Ok(())
    }
}
