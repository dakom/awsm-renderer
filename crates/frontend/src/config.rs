#![allow(dead_code)]
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, LazyLock, Mutex},
};

use crate::{
    pages::app::sidebar::SidebarSection,
    route::{AppRoute, Route},
};
use anyhow::{Context, Result};
use dominator::clone;
use futures_signals::signal::Mutable;
use serde::Deserialize;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;

#[derive(Debug, Clone)]
pub struct Config {
    pub root_path: &'static str,
    pub debug: ConfigDebug,
    pub media_baseurl: String,
    pub gltf_url: String,
    pub generate_mipmaps: bool,
    pub initial_sidebar_open: Option<SidebarSection>,
    pub post_processing_enabled: bool,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config = Config {
        root_path: if cfg!(debug_assertions) {
            ""
        } else {
            "/awsm-renderer"
        },
        debug: if cfg!(debug_assertions) {
            //ConfigDebug::release_mode()
            ConfigDebug::dev_mode(true)
        } else {
            ConfigDebug::release_mode()
        },
        media_baseurl: if cfg!(debug_assertions) {
            "http://localhost:9082".to_string()
        } else {
            "/awsm-renderer/media".to_string()
            //format!("{}/media", web_sys::window().unwrap().origin())
        },
        gltf_url: if cfg!(debug_assertions) {
            "http://localhost:9082/glTF-Sample-Assets/Models".to_string()
        } else {
            "https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Assets/refs/heads/main/Models".to_string()
        },

        generate_mipmaps: true,
        //initial_sidebar_open: Some(SidebarSection::Gltf),
        initial_sidebar_open: Some(SidebarSection::PostProcessing),
        post_processing_enabled: true,
    };

    config
});

#[derive(Debug, Clone)]
pub struct ConfigDebug {
    pub start_route: Arc<Mutex<Option<Route>>>,
}

impl ConfigDebug {
    fn dev_mode(autoconnect: bool) -> Self {
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
