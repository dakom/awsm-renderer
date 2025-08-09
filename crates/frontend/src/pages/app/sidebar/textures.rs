use awsm_renderer::core::texture::{mega_texture::report::MegaTextureSizeReport, TextureFormat};
use awsm_web::file::save::save_file;
use dominator_helpers::futures::{spawn_future, AsyncLoader};

use crate::{
    models::collections::{GltfId, GLTF_SETS},
    pages::app::{context::AppContext, scene::camera::CameraId, sidebar::current_model_signal},
    prelude::*,
};

use super::render_dropdown_label;

pub struct SidebarTextures {
    ctx: AppContext,
    phase: Mutable<Phase>,
    report: Mutex<Option<MegaTextureSizeReport>>,
    to_export: Mutex<Option<(usize, usize)>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Initializing,
    Ready,
    Exporting,
}

impl SidebarTextures {
    pub fn new(ctx: AppContext) -> Arc<Self> {
        Arc::new(Self {
            ctx,
            phase: Mutable::new(Phase::Initializing),
            report: Mutex::new(None),
            to_export: Mutex::new(None),
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("align-items", "center")
            }
        });

        html!("div", {
            .class([&*CONTAINER])
            .future(clone!(state => async move {
                state.initialize().await;
            }))
            .child_signal(state.phase.signal().map(clone!(state => move |phase| {
                Some(match phase {
                    Phase::Initializing => html!("div", {
                        .class([FontSize::Lg.class(), ColorText::Byline.class()])
                        .text("Initializing...")
                    }),
                    Phase::Ready => state.render_inner(),
                    Phase::Exporting => state.render_exporting(),
                })
            })))
        })
    }

    async fn initialize(self: &Arc<Self>) {
        let state = self.clone();

        let renderer = state.ctx.scene.lock_ref();
        let renderer = renderer.as_ref().unwrap().renderer.lock().await;

        let size_report = renderer
            .textures
            .mega_texture
            .size_report(&renderer.gpu.device.limits());

        *state.report.lock().unwrap() = Some(size_report);

        state.phase.set(Phase::Ready);
    }

    fn render_exporting(self: &Arc<Self>) -> Dom {
        let state = self;

        let finished = Mutable::new(false);

        let (atlas_index, layer_index) = state.to_export.lock().unwrap().unwrap().clone();

        html!("div", {
            .future(clone!(state, finished => async move {
                let layer_size = {
                    let report = state.report.lock().unwrap();
                    let report = report.as_ref().unwrap();
                    report.atlas_layer_areas[atlas_index][layer_index].max_size.clone()
                };

                let (gpu, texture_array) = {
                    let renderer = state.ctx.scene.lock_ref();
                    let renderer = renderer.as_ref().unwrap().renderer.lock().await;
                    let gpu = renderer.gpu.clone();
                    let texture_array = renderer.textures.gpu_texture_arrays[atlas_index].clone();
                    (gpu, texture_array)
                };

                let png_data = gpu.export_texture_as_png(
                    &texture_array,
                    layer_size.width,
                    layer_size.height,
                    layer_index as u32,
                    TextureFormat::Rgba16float,
                    true,
                    Some(true)
                ).await;

                match png_data {
                    Ok(png_data) => {
                        tracing::info!("Exported PNG data for Atlas {} - Layer {}: {:?}", atlas_index + 1, layer_index + 1, png_data.len());
                        match save_file(&png_data, &format!("atlas_{}_layer_{}.png", atlas_index + 1, layer_index + 1), Some("image/png")) {
                            Ok(_) => {
                            }
                            Err(err) => {
                                Modal::open(move || {
                                    html!("div", {
                                        .class([FontSize::Lg.class(), ColorText::Error.class()])
                                        .text(&format!("Failed to save PNG data for Atlas {} - Layer {}: {:?}", atlas_index + 1, layer_index + 1, err))
                                    })
                                });
                            }
                        }
                    }
                    Err(err) => {
                        Modal::open(move || {
                            html!("div", {
                                .class([FontSize::Lg.class(), ColorText::Error.class()])
                                .text(&format!("Failed to export PNG data for Atlas {} - Layer {}: {:?}", atlas_index + 1, layer_index + 1, err))
                            })
                        });
                    }
                }

                finished.set(true);

            }))
            .child_signal(finished.signal().map(clone!(state => move |finished| {
                if finished {
                    state.phase.set(Phase::Ready);
                    None
                } else {
                    Some(html!("div", {
                        .class([FontSize::Lg.class(), ColorText::Byline.class()])
                        .text(&format!("Exporting Atlas {} - Layer {}", atlas_index + 1, layer_index + 1))
                    }))
                }
            })))
        })
    }

    fn render_inner(self: &Arc<Self>) -> Dom {
        let state = self;

        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("align-items", "center")
                .style("gap", "3rem")
            }
        });

        static INNER_CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("align-items", "center")
            }
        });
        html!("div", {
            .class(&*CONTAINER)
            .child(html!("div", {
                .class(&*INNER_CONTAINER)
                .child(state.render_export_selector())
                .child(state.render_export_button())
            }))
            .child(state.render_report_button())
        })
    }

    fn render_export_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        let options: Vec<(String, (usize, usize))> = state
            .report
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .atlas_layer_areas
            .iter()
            .enumerate()
            .map(|(atlas_index, atlas)| {
                atlas.iter().enumerate().map(move |(layer_index, area)| {
                    let label = format!("Atlas {} - Layer {}", atlas_index + 1, layer_index + 1);
                    (label, (atlas_index, layer_index))
                })
            })
            .flatten()
            .collect();

        render_dropdown_label(
            "Layer to export",
            Dropdown::new()
                .with_intial_selected(None)
                .with_bg_color(ColorBackground::Dropdown)
                .with_on_change(clone!(state => move |(atlas_index, layer_index)| {
                    *state.to_export.lock().unwrap() = Some((*atlas_index, *layer_index));
                }))
                .with_options(options)
                .render(),
        )
    }

    fn render_export_button(self: &Arc<Self>) -> Dom {
        let state = self;

        Button::new()
            .with_style(ButtonStyle::Outline)
            .with_text("Export")
            .with_on_click(clone!(state => move || {
                state.phase.set(Phase::Exporting);
            }))
            .render()
    }

    fn render_report_button(self: &Arc<Self>) -> Dom {
        let state = self;

        Button::new()
            .with_style(ButtonStyle::Solid)
            .with_text("Show Report")
            .with_on_click(clone!(state => move || {
                let size_report = state.report.lock().unwrap().clone().unwrap();
                Modal::open(move || {
                    html!("pre", {
                        .class([FontSize::Md.class(), ColorText::Paragraph.class()])
                        .text(&format!("Mega Texture Size Report:\n\n{:#?}", size_report))
                    })
                });
            }))
            .render()
    }
}
