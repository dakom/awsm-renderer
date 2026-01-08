#![allow(dead_code)]
use std::sync::{Arc, LazyLock, Mutex};

use crate::{
    pages::app::{
        context::{IblId, SkyboxId},
        sidebar::SidebarSection,
    },
    route::{AppRoute, Route},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub debug: ConfigDebug,
    pub root_base_uri_path: String,
    pub media_base_url_gltf_samples: String,
    pub media_base_url_additional_assets: String,
    pub generate_mipmaps: bool,
    pub initial_sidebar_open: Option<SidebarSection>,
    pub post_processing_enabled: bool,
    pub initial_ibl: IblId,
    pub initial_skybox: SkyboxId,
    pub cache_buster: bool,
    pub initial_show_grid: bool,
    pub initial_show_gizmo_translation: bool,
    pub initial_show_gizmo_rotation: bool,
    pub initial_show_gizmo_scale: bool,
}

#[allow(clippy::option_env_unwrap)]
pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config {
        media_base_url_gltf_samples: option_env!("MEDIA_BASE_URL_GLTF_SAMPLES")
            .expect("MEDIA_BASE_URL_GLTF_SAMPLES must be set")
            .to_string(),
        media_base_url_additional_assets: option_env!("MEDIA_BASE_URL_ADDITIONAL_ASSETS")
            .expect("MEDIA_BASE_URL_ADDITIONAL_ASSETS must be set")
            .to_string(),
        root_base_uri_path: option_env!("ROOT_BASE_URI_PATH")
            .expect("ROOT_BASE_URI_PATH must be set")
            .to_string(),
        debug: if cfg!(debug_assertions) {
            //ConfigDebug::release_mode()
            ConfigDebug::dev_mode()
        } else {
            ConfigDebug::release_mode()
        },

        generate_mipmaps: true,
        //initial_sidebar_open: Some(SidebarSection::Gltf),
        initial_sidebar_open: Some(SidebarSection::PostProcessing),
        post_processing_enabled: true,
        initial_ibl: IblId::default(),
        initial_skybox: SkyboxId::default(),
        cache_buster: cfg!(debug_assertions),
        initial_show_grid: false,
        initial_show_gizmo_translation: true,
        initial_show_gizmo_rotation: true,
        initial_show_gizmo_scale: true,
    }
});

#[derive(Debug, Clone)]
pub struct ConfigDebug {
    pub start_route: Arc<Mutex<Option<Route>>>,
}

impl ConfigDebug {
    fn dev_mode() -> Self {
        Self {
            start_route: Arc::new(Mutex::new(Some(Route::App(AppRoute::Init)))),
        }
    }

    fn release_mode() -> Self {
        Self {
            start_route: Arc::new(Mutex::new(None)),
        }
    }
}
