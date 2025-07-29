use awsm_renderer::render::post_process::{PostProcessSettings, ToneMapping};

use crate::{pages::app::sidebar::material::FragmentShaderKind, prelude::*};

use super::scene::{camera::CameraId, AppScene};

#[derive(Clone)]
pub struct AppContext {
    pub camera_id: Mutable<CameraId>,
    pub shader: Mutable<FragmentShaderKind>,
    pub scene: Mutable<Option<Arc<AppScene>>>,
    pub generate_mipmaps: Mutable<bool>,
    pub post_processing: MutablePostProcessingSettings,
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            camera_id: Mutable::new(CameraId::default()),
            shader: Mutable::new(FragmentShaderKind::Pbr),
            scene: Mutable::new(None),
            generate_mipmaps: Mutable::new(CONFIG.generate_mipmaps),
            post_processing: MutablePostProcessingSettings::default(),
        }
    }
}

#[derive(Clone)]
pub struct MutablePostProcessingSettings {
    pub tonemapping: Mutable<Option<ToneMapping>>,
    pub gamma_correction: Mutable<bool>,
    pub anti_aliasing: Mutable<bool>,
}

impl MutablePostProcessingSettings {
    pub fn signal(&self) -> impl Signal<Item = PostProcessSettings> {
        map_ref! {
            let tonemapping = self.tonemapping.signal(),
            let gamma_correction = self.gamma_correction.signal(),
            let anti_aliasing = self.anti_aliasing.signal()
            => PostProcessSettings {
                enabled: CONFIG.post_processing_enabled,
                tonemapping: *tonemapping,
                gamma_correction: *gamma_correction,
                anti_aliasing: *anti_aliasing,
            }
        }
    }
}

impl From<MutablePostProcessingSettings> for PostProcessSettings {
    fn from(settings: MutablePostProcessingSettings) -> Self {
        PostProcessSettings {
            enabled: CONFIG.post_processing_enabled,
            tonemapping: settings.tonemapping.get(),
            gamma_correction: settings.gamma_correction.get(),
            anti_aliasing: settings.anti_aliasing.get(),
        }
    }
}

impl Default for MutablePostProcessingSettings {
    fn default() -> Self {
        Self {
            tonemapping: Mutable::new(Some(ToneMapping::KhronosPbrNeutral)),
            gamma_correction: Mutable::new(true),
            anti_aliasing: Mutable::new(true),
        }
    }
}
