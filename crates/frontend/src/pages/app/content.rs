use crate::{models::collections::GltfId, prelude::*};

pub struct AppContent {}

impl AppContent {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
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
        html!("div", {
            .class(&*CONTAINER)
            .child_signal(Route::signal().map(clone!(state => move |route| {
                Some(match route {
                    Route::App(AppRoute::Model(model_id)) => state.render_model(model_id),
                    _ => html!("div", {
                        .class([FontSize::H3.class(), ColorText::Paragraph.class()])
                        .text("Select a model from the sidebar")
                    })
                })
            })))
        })
    }

    fn render_model(self: &Arc<Self>, model_id: GltfId) -> Dom {
        html!("div", {
            .class([FontSize::H3.class(), ColorText::Paragraph.class()])
            .text(&format!("Rendering model with ID: {}", model_id))
        })
    }
}
