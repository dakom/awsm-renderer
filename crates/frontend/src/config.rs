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
    pub root_path: &'static str,
    pub debug: ConfigDebug,
    pub media_baseurl: String,
    pub gltf_samples_url: String,
    pub additional_assets_url: String,
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

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config {
        root_path: if cfg!(debug_assertions) {
            ""
        } else {
            "/awsm-renderer"
        },
        debug: if cfg!(debug_assertions) {
            //ConfigDebug::release_mode()
            ConfigDebug::dev_mode()
        } else {
            ConfigDebug::release_mode()
        },
        media_baseurl: if cfg!(debug_assertions) {
            "http://localhost:9082".to_string()
        } else {
            "/awsm-renderer/media".to_string()
            //format!("{}/media", web_sys::window().unwrap().origin())
        },
        gltf_samples_url: if cfg!(debug_assertions) {
            "http://localhost:9082/glTF-Sample-Assets/Models".to_string()
        } else {
            "https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Assets/refs/heads/main/Models".to_string()
        },
        additional_assets_url: if cfg!(debug_assertions) {
            "http://localhost:9083".to_string()
        } else {
            "https://dakom.github.io/awsm-renderer-assets".to_string()
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
