use awsm_renderer::{AwsmRendererBuilder, AwsmRendererLogging};
use awsm_web::dom::resize::{self, ResizeObserver};
use wasm_bindgen_futures::spawn_local;

use crate::{models::collections::GltfId, pages::app::sidebar::current_model_signal, prelude::*};

use super::{context::AppContext, scene::AppScene};

pub struct AppCanvas {
    pub ctx: AppContext,
    pub display_text: Mutable<String>,
}

impl AppCanvas {
    pub fn new(ctx: AppContext) -> Arc<Self> {
        Arc::new(Self {
            ctx,
            display_text: Mutable::new("<-- Select a model from the sidebar".to_string()),
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;

        static FULL_AREA: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("margin", "0")
                .style("padding", "0")
                .style("position", "absolute")
                .style("top", "0")
                .style("left", "0")
                .style("width", "100%")
                .style("height", "100%")
            }
        });

        let sig = map_ref! {
            let model_id = current_model_signal(),
            let scene = state.ctx.scene.signal_cloned()
            => {
                match (model_id, scene) {
                    (Some(model_id), Some(scene)) => {
                        Some((model_id.clone(), scene.clone()))
                    }
                    _ => {
                        None
                    }
                }
            }
        };

        html!("div", {
            .style("position", "relative")
            .style("width", "100%")
            .style("height", "100%")
            .child(html!("canvas" => web_sys::HtmlCanvasElement, {
                .class(&*CURSOR_POINTER)
                .class(&*FULL_AREA)
                .after_inserted(clone!(state => move |canvas| {
                    spawn_local(clone!(state => async move {
                        let renderer = AwsmRendererBuilder::new(web_sys::window().unwrap().navigator().gpu())
                            .with_logging(AwsmRendererLogging { render_timings: true })
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

                        state.ctx.scene.set(Some(AppScene::new(state.ctx.clone(), renderer)));
                    }));
                }))
            }))
            .child(html!("div", {
                .class(&*FULL_AREA)
                .style("pointer-events", "none")
                .child(html!("div", {
                    .style("padding", "1rem")
                    .class([FontSize::H3.class(), ColorText::GltfContent.class(), &*USER_SELECT_NONE])
                    .text_signal(state.display_text.signal_cloned())
                }))
            }))
            .future(sig.for_each(clone!(state => move |data| {
                clone!(state => async move {
                    if let Some((gltf_id, scene)) = data {
                        state.display_text.set(format!("Loading: {}", gltf_id));

                        scene.clear().await;

                        let loader = match scene.load(gltf_id.clone()).await {
                            Ok(loader) => loader,
                            Err(err) => {
                                tracing::error!("{:?}", err);
                                state.display_text.set(format!("Error loading: {}", gltf_id));
                                return;
                            }
                        };

                        state.display_text.set(format!("Uploading data: {}", gltf_id));

                        let data = match scene.upload_data(gltf_id, loader).await {
                            Ok(data) => data,
                            Err(err) => {
                                tracing::error!("{:?}", err);
                                state.display_text.set(format!("Error uploading data: {}", gltf_id));
                                return;
                            }
                        };

                        state.display_text.set(format!("Populating data: {}", gltf_id));

                        if let Err(err) = scene.populate(data).await {
                            tracing::error!("{:?}", err);
                            state.display_text.set(format!("Error populating data: {}", gltf_id));
                            return;
                        }

                        state.display_text.set(format!("Setting up scene: {}", gltf_id));

                        scene.setup().await;

                        if let Err(err) = scene.render().await {
                            tracing::error!("{:?}", err);
                            state.display_text.set(format!("Error rendering: {}", gltf_id));
                            return;
                        }

                        state.display_text.set(format!("Now showing: {}", gltf_id));


                        scene.start_animation_loop();
                    }
                })
            })))
        })
    }
}
