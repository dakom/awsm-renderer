use awsm_renderer::core::{command::color::Color, cubemap::images::CubemapBitmapColors};

use crate::{pages::app::sidebar::material::FragmentShaderKind, prelude::*};

use super::scene::{camera::CameraId, AppScene};

#[derive(Clone)]
pub struct AppContext {
    pub camera_id: Mutable<CameraId>,
    pub scene: Mutable<Option<Arc<AppScene>>>,
    pub generate_mipmaps: Mutable<bool>,
    pub material: MutableMaterial,
    pub ibl_id: Mutable<IblId>,
    pub skybox_id: Mutable<SkyboxId>,
}

#[derive(Clone)]
pub struct MutableMaterial {
    pub debug_normals: Mutable<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum IblId {
    #[default]
    PhotoStudio,
    AllWhite,
}

impl IblId {
    pub fn path(&self) -> Option<&'static str> {
        match self {
            IblId::PhotoStudio => Some("photo_studio"),
            IblId::AllWhite => None,
        }
    }

    pub fn cubemap_colors(&self) -> Option<CubemapBitmapColors> {
        match self {
            IblId::PhotoStudio => None,
            IblId::AllWhite => Some(CubemapBitmapColors::all(Color::WHITE)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SkyboxId {
    #[default]
    SameAsIbl,
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
            ibl_id: Mutable::new(CONFIG.initial_ibl.clone()),
            skybox_id: Mutable::new(CONFIG.initial_skybox.clone()),
        }
    }
}
