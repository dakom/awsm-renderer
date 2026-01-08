use crate::prelude::*;

use super::scene::{camera::CameraId, AppScene};

#[derive(Clone)]
pub struct AppContext {
    pub camera_id: Mutable<CameraId>,
    pub scene: Mutable<Option<Arc<AppScene>>>,
    pub material: MutableMaterial,
    pub ibl_id: Mutable<IblId>,
    pub skybox_id: Mutable<SkyboxId>,
    pub editor_grid_enabled: Mutable<bool>,
    pub editor_gizmo_translation_enabled: Mutable<bool>,
    pub editor_gizmo_rotation_enabled: Mutable<bool>,
    pub editor_gizmo_scale_enabled: Mutable<bool>,
}

#[derive(Clone)]
pub struct MutableMaterial {
    pub debug_normals: Mutable<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum IblId {
    #[default]
    PhotoStudio,
    SimpleSky,
    AllWhite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SkyboxId {
    #[default]
    SameAsIbl,
    // Not a real mode, just for debugging to use original default from renderer
    None,
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            camera_id: Mutable::new(CameraId::default()),
            scene: Mutable::new(None),
            material: MutableMaterial {
                debug_normals: Mutable::new(false),
            },
            ibl_id: Mutable::new(CONFIG.initial_ibl),
            skybox_id: Mutable::new(CONFIG.initial_skybox),
            editor_grid_enabled: Mutable::new(CONFIG.initial_show_grid),
            editor_gizmo_translation_enabled: Mutable::new(CONFIG.initial_show_gizmo_translation),
            editor_gizmo_rotation_enabled: Mutable::new(CONFIG.initial_show_gizmo_rotation),
            editor_gizmo_scale_enabled: Mutable::new(CONFIG.initial_show_gizmo_scale),
        }
    }
}
