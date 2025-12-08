#![allow(dead_code)]
#![allow(clippy::arc_with_non_send_sync)]
#![allow(clippy::type_complexity)]
mod atoms;
mod config;
mod header;
mod logger;
mod models;
mod pages;
mod prelude;
mod route;
mod theme;
mod util;

use header::Header;
use pages::{app::AppUi, not_found::NotFoundUi};
use prelude::*;

pub fn main() {
    wasm_bindgen_futures::spawn_local(async {
        init().await;
    });
}

async fn init() {
    logger::init_logger();
    theme::stylesheet::init();

    if let Some(init_url) = CONFIG.debug.start_route.lock().unwrap_throw().take() {
        init_url.go_to_url();
    }

    dominator::append_dom(
        &dominator::body(),
        html!("div", {
            .child_signal(TopLevelRoute::signal().map(|route| {
                match route {
                    TopLevelRoute::App => None,
                    _ => Some(Header::new().render()),
                }
            }))
            .child_signal(TopLevelRoute::signal().map(|route| {
                Some(match route {
                    TopLevelRoute::App => AppUi::new().render(),
                    TopLevelRoute::NotFound => NotFoundUi::new().render()
                })
            }))
            .fragment(&Modal::render())
        }),
    );
}
