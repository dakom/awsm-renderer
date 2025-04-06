use awsm_renderer::AwsmRendererBuilder;
use awsm_web::dom::resize::{self, ResizeObserver};
use wasm_bindgen_futures::spawn_local;

use crate::{models::collections::GltfId, pages::app::sidebar::current_model_signal, prelude::*};

use super::renderer::AppRenderer;

pub struct AppCanvas {
    pub resize_observer: Arc<Mutex<Option<ResizeObserver>>>,
    pub renderer: Mutable<Option<Arc<AppRenderer>>>,
}

impl AppCanvas {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            resize_observer: Arc::new(Mutex::new(None)),
            renderer: Mutable::new(None),
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

        let sig = map_ref! {
            let model_id = current_model_signal(),
            let renderer = state.renderer.signal_cloned()
            => {
                match (model_id, renderer) {
                    (Some(model_id), Some(renderer)) => {
                        Some((model_id.clone(), renderer.clone()))
                    }
                    _ => {
                        None
                    }
                }
            }
        };

        html!("div", {
            .class(&*FULL_AREA)
            .style("position", "relative")
            .child(html!("canvas" => web_sys::HtmlCanvasElement, {
                .after_inserted(clone!(state => move |canvas| {
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

                    spawn_local(clone!(state => async move {
                        let renderer = AwsmRendererBuilder::new(web_sys::window().unwrap().navigator().gpu())
                            .init_adapter()
                            .await
                            .unwrap()
                            .init_device()
                            .await
                            .unwrap()
                            .init_context(canvas.clone())
                            .unwrap()
                            .build()
                            .unwrap();

                        state.renderer.set(Some(AppRenderer::new(renderer)));
                    }));
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
            .future(sig.for_each(clone!(state => move |data| {
                async move {
                    if let Some((gltf_id, renderer)) = data {
                        if let Err(err) = renderer.render(gltf_id).await {
                            tracing::error!("{:?}", err);
                        }
                    }
                }
            })))
        })
    }
}
