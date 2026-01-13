use awsm_renderer::post_process::ToneMapping;
use wasm_bindgen_futures::spawn_local;

use crate::{
    pages::app::{context::AppContext, sidebar::render_dropdown_label},
    prelude::*,
};

pub struct SidebarProcessing {
    ctx: AppContext,
}

impl SidebarProcessing {
    pub fn new(ctx: AppContext) -> Arc<Self> {
        Arc::new(Self { ctx })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;

        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("align-items", "flex-start")
                .style("gap", "1rem")
            }
        });

        html!("div", {
            .class(&*CONTAINER)
            .child(state.render_tonemapping_selector())
            .child(state.render_msaa_selector())
            .child(state.render_smaa_selector())
            // .child(state.render_tonemap_selector())
            // .child(state.render_gamma_selector())
            // .child(state.render_anti_alias_selector())
        })
    }

    fn render_msaa_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        render_dropdown_label(
            "MSAA Anti-Aliasing",
            Dropdown::new()
                .with_intial_selected(Some(
                    state
                        .ctx
                        .anti_alias
                        .get_cloned()
                        .msaa_sample_count
                        .is_some(),
                ))
                .with_bg_color(ColorBackground::Dropdown)
                .with_on_change(clone!(state => move |msaa| {
                    let mut anti_alias = state.ctx.anti_alias.get_cloned();
                    anti_alias.msaa_sample_count = if *msaa {
                        Some(4)
                    } else {
                        None
                    };
                    state.ctx.anti_alias.set_neq(anti_alias);

                    spawn_local(clone!(state => async move {
                        if let Some(scene) = state.ctx.scene.get_cloned() {
                            if let Err(err) = scene.reset_anti_aliasing().await {
                                tracing::error!("Error resetting anti_aliasing: {}", err);
                            }
                        }
                    }));
                }))
                .with_options([("On".to_string(), true), ("Off".to_string(), false)])
                .render(),
        )
    }

    fn render_smaa_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        render_dropdown_label(
            "SMAA Anti-Aliasing",
            Dropdown::new()
                .with_intial_selected(Some(state.ctx.anti_alias.get_cloned().smaa))
                .with_bg_color(ColorBackground::Dropdown)
                .with_on_change(clone!(state => move |smaa| {
                    let mut anti_alias = state.ctx.anti_alias.get_cloned();
                    anti_alias.smaa = *smaa;
                    state.ctx.anti_alias.set_neq(anti_alias);

                    spawn_local(clone!(state => async move {
                        if let Some(scene) = state.ctx.scene.get_cloned() {
                            if let Err(err) = scene.reset_anti_aliasing().await {
                                tracing::error!("Error resetting anti_aliasing: {}", err);
                            }
                        }
                    }));
                }))
                .with_options([("On".to_string(), true), ("Off".to_string(), false)])
                .render(),
        )
    }

    fn render_tonemapping_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        render_dropdown_label(
            "Tonemapping",
            Dropdown::new()
                .with_intial_selected(Some(state.ctx.post_processing.get_cloned().tonemapping))
                .with_bg_color(ColorBackground::Dropdown)
                .with_on_change(clone!(state => move |tonemapping| {
                    let mut post_procesing = state.ctx.post_processing.get_cloned();
                    post_procesing.tonemapping = *tonemapping;
                    state.ctx.post_processing.set_neq(post_procesing);

                    spawn_local(clone!(state => async move {
                        if let Some(scene) = state.ctx.scene.get_cloned() {
                            if let Err(err) = scene.reset_post_processing().await {
                                tracing::error!("Error resetting post_processing: {}", err);
                            }
                        }
                    }));
                }))
                .with_options([
                    ("None".to_string(), ToneMapping::None),
                    (
                        "Khronos PBR Neutral".to_string(),
                        ToneMapping::KhronosNeutralPbr,
                    ),
                ])
                .render(),
        )
    }

    // fn render_tonemap_selector(self: &Arc<Self>) -> Dom {
    //     let state = self;

    //     render_dropdown_label(
    //         "Tonemapping",
    //         Dropdown::new()
    //             .with_intial_selected(Some(state.ctx.post_processing.tonemapping.get()))
    //             .with_bg_color(ColorBackground::Dropdown)
    //             .with_on_change(clone!(state => move |tonemapping| {
    //                 //state.ctx.post_processing.tonemapping.set_neq(*tonemapping);
    //             }))
    //             .with_options([
    //                 (
    //                     "Khronos PBR Neutral".to_string(),
    //                     Some(ToneMapping::KhronosPbrNeutral),
    //                 ),
    //                 ("Agx".to_string(), Some(ToneMapping::Agx)),
    //                 ("Filmic".to_string(), Some(ToneMapping::Filmic)),
    //                 ("None".to_string(), None),
    //             ])
    //             .render(),
    //     )
    // }

    // fn render_gamma_selector(self: &Arc<Self>) -> Dom {
    //     let state = self;

    //     Checkbox::new(CheckboxStyle::Dark)
    //         .with_content_after(html!("span", {
    //             .text("Gamma correction")
    //         }))
    //         .with_selected_signal(state.ctx.post_processing.gamma_correction.signal())
    //         .with_on_click(clone!(state => move || {
    //             state.ctx.post_processing.gamma_correction.set_neq(!state.ctx.post_processing.gamma_correction.get());
    //         }))
    //         .render()
    // }

    // fn render_anti_alias_selector(self: &Arc<Self>) -> Dom {
    //     let state = self;

    //     Checkbox::new(CheckboxStyle::Dark)
    //         .with_content_after(html!("span", {
    //             .text("Anti-aliasing")
    //         }))
    //         .with_selected_signal(state.ctx.post_processing.anti_aliasing.signal())
    //         .with_on_click(clone!(state => move || {
    //             state.ctx.post_processing.anti_aliasing.set_neq(!state.ctx.post_processing.anti_aliasing.get());
    //         }))
    //         .render()
    // }
}
