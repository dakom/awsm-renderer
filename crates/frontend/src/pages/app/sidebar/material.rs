use awsm_renderer::shaders::FragmentShaderKind;

use crate::{
    models::collections::{GltfId, GLTF_SETS},
    pages::app::{context::AppContext, scene::camera::CameraId, sidebar::current_model_signal},
    prelude::*,
};

use super::render_dropdown_label;

pub struct SidebarMaterial {
    ctx: AppContext,
}

impl SidebarMaterial {
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
            .child(state.render_shader_selector())
        })
    }

    fn render_shader_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        render_dropdown_label(
            "Shader",
            Dropdown::new()
                .with_intial_selected(Some(state.ctx.shader.get()))
                .with_bg_color(ColorBackground::Dropdown)
                .with_on_change(clone!(state => move |shader| {
                    state.ctx.shader.set_neq(*shader);
                }))
                .with_options([
                    ("PBR".to_string(), FragmentShaderKind::Pbr),
                    (
                        "Debug Normals".to_string(),
                        FragmentShaderKind::DebugNormals,
                    ),
                ])
                .render(),
        )
    }
}
