use awsm_web::dom::resize::{self, ResizeObserver};

use crate::{models::collections::GltfId, prelude::*};

use super::renderer::AppRenderer;

pub struct AppContent {
    pub renderer: AppRenderer,
    pub resize_observer: Arc<Mutex<Option<ResizeObserver>>>,
}

impl AppContent {
    pub fn new(renderer: AppRenderer) -> Arc<Self> {
        Arc::new(Self {
            renderer,
            resize_observer: Arc::new(Mutex::new(None)),
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;

        static FULL_AREA: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("top", "0")
                .style("left", "0")
                .style("width", "100%")
                .style("height", "100%")
            }
        });
        html!("div", {
            .future(Route::signal().for_each(clone!(state => move |route| {
                clone!(state => async move {
                    match route {
                        Route::App(AppRoute::Model(model_id)) => {
                            state.renderer.set_model(model_id);
                        }
                        _ => { }
                    }
                })
            })))
            .class(&*FULL_AREA)
            .style("position", "relative")
            .child(html!("canvas" => web_sys::HtmlCanvasElement, {
                .after_inserted(clone!(state => move |canvas| {
                    state.renderer.set_canvas(canvas.clone());

                    let resize_observer = ResizeObserver::new(
                        clone!(canvas => move |entries| {
                            if let Some(entry) = entries.get(0) {
                                let width = entry.content_box_sizes[0].inline_size;
                                let height = entry.content_box_sizes[0].block_size;
                                canvas.set_width(width);
                                canvas.set_height(height);
                            }
                        }),
                        None
                    );

                    resize_observer.observe(&canvas);

                    *state.resize_observer.lock().unwrap() = Some(resize_observer);
                }))
                .class(&*FULL_AREA)
                .style("position", "absolute")
            }))
            .child(html!("div", {
                .class(&*FULL_AREA)
                .style("position", "absolute")
                .style("padding", "1rem")
                .class([FontSize::H3.class(), ColorText::GltfContent.class()])
                .text_signal(Route::signal().map(clone!(state => move |route| {
                    match route {
                        Route::App(AppRoute::Model(model_id)) => {
                            format!("Now showing: {}", model_id)
                        }
                        _ => {
                            "<-- Select a model from the sidebar".to_string()
                        }
                    }
                })))
            }))
        })
    }
}
