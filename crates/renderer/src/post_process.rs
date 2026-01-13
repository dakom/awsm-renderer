use crate::{error::Result, AwsmRenderer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostProcessing {
    pub tonemapping: ToneMapping,
}

#[derive(Clone, Debug, PartialEq, Eq, Copy, Hash)]
pub enum ToneMapping {
    None,
    KhronosNeutralPbr,
}

impl Default for PostProcessing {
    fn default() -> Self {
        Self {
            tonemapping: ToneMapping::KhronosNeutralPbr,
        }
    }
}

impl AwsmRenderer {
    pub async fn set_post_processing(&mut self, pp: PostProcessing) -> Result<()> {
        self.post_processing = pp;

        self.render_passes
            .display
            .pipelines
            .set_render_pipeline_key(
                &self.anti_aliasing,
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
