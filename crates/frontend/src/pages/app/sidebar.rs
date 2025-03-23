use crate::{
    models::collections::{GltfId, GLTF_SETS},
    prelude::*,
};

use super::renderer::AppRenderer;

pub struct AppSidebar {
    pub renderer: AppRenderer,
}

impl AppSidebar {
    pub fn new(renderer: AppRenderer) -> Arc<Self> {
        Arc::new(Self { renderer })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("margin", "1rem")
            }
        });

        let ordered_keys = [
            "Todo",
            // "Minimal",
            // "Standard",
            // "Extension Tests",
            // "Standard",
            // "Feature tests",
        ];

        for key in ordered_keys {
            if !GLTF_SETS.contains_key(key) {
                tracing::error!("Key not found in GLTF_SETS: {}", key);
            }
        }

        for key in GLTF_SETS.keys() {
            if !ordered_keys.contains(&key) {
                tracing::error!("Key not found in ordered_keys: {}", key);
            }
        }

        html!("div", {
            .class(&*CONTAINER)
            .child(html!("div", {
                .child(html!("div", {
                    .class([FontSize::H3.class(), ColorText::SidebarHeader.class()])
                    .text("Choose a GLTF Model")
                }))
                .child_signal(current_model_signal().map(clone!(state => move |model_id| {
                    Some(html!("div", {
                        .children(
                            ordered_keys.iter().map(|set_name| {
                                state.render_gltf_selector(set_name, model_id)
                            }).collect::<Vec<_>>()
                        )
                    }))
                })))
            }))
        })
    }

    fn render_gltf_selector(
        self: &Arc<Self>,
        set_name: &'static str,
        initial_selected: Option<GltfId>,
    ) -> Dom {
        let state = self;

        let options = GLTF_SETS
            .get(set_name)
            .unwrap_throw()
            .into_iter()
            .map(|gltf_id| (gltf_id.label().to_string(), *gltf_id))
            .collect::<Vec<_>>();

        let initial_selected = initial_selected.and_then(|initial_selected| {
            options.iter().find_map(|(_, id)| {
                if *id == initial_selected {
                    Some(*id)
                } else {
                    None
                }
            })
        });

        render_dropdown_label(
            set_name,
            Dropdown::new()
                .with_intial_selected(initial_selected)
                .with_bg_color(ColorBackground::Dropdown)
                .with_on_change(clone!(state => move |id| {
                    Route::App(AppRoute::Model(*id)).go_to_url();
                }))
                .with_options(options)
                .render(),
        )
    }
}

fn render_dropdown_label(label: &str, dropdown: Dom) -> Dom {
    static CONTAINER: LazyLock<String> = LazyLock::new(|| {
        class! {
            .style("display", "flex")
            .style("flex-direction", "column")
            .style("margin", "1rem")
            .style("gap", "1rem")
        }
    });

    html!("div", {
        .class(&*CONTAINER)
        .child(html!("div", {
            .class([FontSize::Xlg.class(), ColorText::SidebarHeader.class()])
            .text(label)
        }))
        .child(dropdown)
    })
}

fn current_model_signal() -> impl Signal<Item = Option<GltfId>> {
    Route::signal().map(|route| match route {
        Route::App(AppRoute::Model(model_id)) => Some(model_id),
        _ => None,
    })
}
