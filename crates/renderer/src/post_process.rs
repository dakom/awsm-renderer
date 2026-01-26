//! Post-processing configuration and updates.

use crate::{error::Result, AwsmRenderer};

/// Post-processing settings for the renderer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostProcessing {
    pub tonemapping: ToneMapping,
    pub bloom: bool,
    pub dof: bool,
}

/// Tonemapping operator selection.
#[derive(Clone, Debug, PartialEq, Eq, Copy, Hash)]
pub enum ToneMapping {
    None,
    KhronosNeutralPbr,
    Aces,
}

impl Default for PostProcessing {
    fn default() -> Self {
        Self {
            tonemapping: ToneMapping::KhronosNeutralPbr,
            bloom: false,
            dof: false,
        }
    }
}

impl AwsmRenderer {
    /// Applies post-processing configuration and rebuilds pipelines as needed.
    pub async fn set_post_processing(&mut self, pp: PostProcessing) -> Result<()> {
        self.post_processing = pp;

        self.render_passes
            .effects
            .pipelines
            .set_render_pipeline_keys(
                &self.anti_aliasing,
                &self.post_processing,
                &self.gpu,
                &mut self.shaders,
                &mut self.pipelines,
                &self.pipeline_layouts,
                &self.render_textures.formats,
            )
            .await?;

        self.render_passes
            .display
            .pipelines
            .set_render_pipeline_key(
                &self.post_processing,
                &self.gpu,
                &mut self.shaders,
                &mut self.pipelines,
                &self.pipeline_layouts,
                &self.render_textures.formats,
            )
            .await?;
        Ok(())
    }
}
