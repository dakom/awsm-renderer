use awsm_renderer::shaders::FragmentShaderKind;

use crate::{
    atoms::checkbox::{Checkbox, CheckboxStyle}, models::collections::{GltfId, GLTF_SETS}, pages::app::{context::AppContext, scene::camera::CameraId, sidebar::{current_model_signal, render_checkbox_label}}, prelude::*
};

use super::render_dropdown_label;

pub struct SidebarPostProcessing {
    ctx: AppContext,
}

impl SidebarPostProcessing {
    pub fn new(ctx: AppContext) -> Arc<Self> {
        Arc::new(Self { ctx })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
            }
        });

        html!("div", {
            .class(&*CONTAINER)
            .child(state.render_tonemap_selector())
            .child(state.render_gamma_selector())
        })
    }

    fn render_tonemap_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        render_dropdown_label(
            "Tonemapping",
            Dropdown::new()
                .with_intial_selected(Some(state.ctx.shader.get()))
                .with_bg_color(ColorBackground::Dropdown)
                .with_on_change(clone!(state => move |shader| {
                    state.ctx.shader.set_neq(*shader);
                }))
                .with_options([
                    ("Khronos PBR Neutral".to_string(), FragmentShaderKind::Pbr), 
                    ("Agx".to_string(), FragmentShaderKind::Pbr), 
                    ("Filmic".to_string(), FragmentShaderKind::Pbr), 
                    ("None".to_string(), FragmentShaderKind::Pbr), 
                ])
                .render(),
        )
    }

    fn render_gamma_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        Checkbox::new(CheckboxStyle::Dark)
            .with_content_after(html!("span", {
                .text("Gamma correction")
            }))
            .with_selected_signal(state.ctx.post_processing.gamma_correction.signal())
            .with_on_click(clone!(state => move || {
                state.ctx.post_processing.gamma_correction.set_neq(!state.ctx.post_processing.gamma_correction.get());
            }))
            .render()
    }
}