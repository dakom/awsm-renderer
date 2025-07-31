use crate::{pages::app::sidebar::material::FragmentShaderKind, prelude::*};

use super::scene::{camera::CameraId, AppScene};

#[derive(Clone)]
pub struct AppContext {
    pub camera_id: Mutable<CameraId>,
    pub scene: Mutable<Option<Arc<AppScene>>>,
    pub generate_mipmaps: Mutable<bool>,
    pub material: MutableMaterial,
}

#[derive(Clone)]
pub struct MutableMaterial {
    pub debug_normals: Mutable<bool>,
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            camera_id: Mutable::new(CameraId::default()),
            scene: Mutable::new(None),
            generate_mipmaps: Mutable::new(CONFIG.generate_mipmaps),
            material: MutableMaterial {
                debug_normals: Mutable::new(false),
            },
        }
    }
}
