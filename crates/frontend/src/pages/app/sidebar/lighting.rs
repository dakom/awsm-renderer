use wasm_bindgen_futures::spawn_local;

use crate::{
    atoms::checkbox::{Checkbox, CheckboxStyle},
    pages::app::context::{AppContext, IblId},
    prelude::*,
};

use super::render_dropdown_label;

pub struct SidebarLighting {
    ctx: AppContext,
}

impl SidebarLighting {
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
            .child(state.render_ibl_selector())
            .child(state.render_punctual_lights_selector())
        })
    }

    fn render_ibl_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        render_dropdown_label(
            "IBL Environment",
            Dropdown::new()
                .with_intial_selected(Some(state.ctx.ibl_id.get()))
                .with_bg_color(ColorBackground::Dropdown)
                .with_on_change(clone!(state => move |ibl_id| {
                    state.ctx.ibl_id.set_neq(*ibl_id);
                }))
                .with_options([
                    ("Photo Studio".to_string(), IblId::PhotoStudio),
                    ("All White".to_string(), IblId::AllWhite),
                    ("Simple Sky".to_string(), IblId::SimpleSky),
                ])
                .render(),
        )
    }

    fn render_punctual_lights_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        Checkbox::new(CheckboxStyle::Dark)
            .with_content_after(html!("span", {
                .text("Punctual Lights")
            }))
            .with_selected_signal(state.ctx.punctual_lights.signal())
            .with_on_click(clone!(state => move || {
                state.ctx.punctual_lights.set_neq(!state.ctx.punctual_lights.get());

                spawn_local(clone!(state => async move {
                    if let Some(scene) = state.ctx.scene.get_cloned() {
                        if let Err(err) = scene.reset_punctual_lights().await {
                            tracing::error!("Error resetting punctual lights: {}", err);
                        }
                    }
                }));
            }))
            .render()
    }
}
