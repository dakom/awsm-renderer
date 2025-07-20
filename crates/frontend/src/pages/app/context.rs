use awsm_renderer::shaders::FragmentShaderKind;

use crate::prelude::*;

use super::scene::{camera::CameraId, AppScene};

#[derive(Clone)]
pub struct AppContext {
    pub camera_id: Mutable<CameraId>,
    pub shader: Mutable<FragmentShaderKind>,
    pub scene: Mutable<Option<Arc<AppScene>>>,
    pub generate_mipmaps: Mutable<bool>,
    pub post_processing: PostProcessingSettings,
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            camera_id: Mutable::new(CameraId::default()),
            shader: Mutable::new(FragmentShaderKind::Pbr),
            scene: Mutable::new(None),
            generate_mipmaps: Mutable::new(CONFIG.generate_mipmaps),
            post_processing: PostProcessingSettings::default()
        }
    }
}

#[derive(Clone)]
pub struct PostProcessingSettings {
    pub tonemap: Mutable<FragmentShaderKind>,
    pub gamma_correction: Mutable<bool>,
}

impl Default for PostProcessingSettings {
    fn default() -> Self {
        Self {
            tonemap: Mutable::new(FragmentShaderKind::Pbr),
            gamma_correction: Mutable::new(false),
        }
    }
}
