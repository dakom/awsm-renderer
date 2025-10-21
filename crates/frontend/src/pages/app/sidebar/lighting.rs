use crate::{
    atoms::checkbox::{Checkbox, CheckboxStyle},
    models::collections::{GltfId, GLTF_SETS},
    pages::app::{
        context::{AppContext, IblId},
        scene::camera::CameraId,
        sidebar::{current_model_signal, render_checkbox_label},
    },
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
                ])
                .render(),
        )
    }
}
