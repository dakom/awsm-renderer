#![allow(dead_code)]
use std::{collections::{BTreeMap, HashMap}, sync::{Arc, LazyLock, Mutex}};

use crate::route::{AppRoute, Route};
use anyhow::{Result, Context};
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
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config = Config {
        root_path: "",
        debug: if cfg!(debug_assertions) {
            //ConfigDebug::release_mode()
            ConfigDebug::dev_mode(true)
        } else {
            ConfigDebug::release_mode()
        },
        media_baseurl: if cfg!(debug_assertions) {
            "http://localhost:9082".to_string()
        } else {
            format!("{}/media", web_sys::window().unwrap().origin())
        },
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
