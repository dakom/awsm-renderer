use crate::{
    atoms::checkbox::{Checkbox, CheckboxStyle},
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
            .child(state.render_debug_normals())
        })
    }

    fn render_debug_normals(self: &Arc<Self>) -> Dom {
        let state = self;

        Checkbox::new(CheckboxStyle::Dark)
            .with_content_after(html!("span", {
                .text("Debug Normals")
            }))
            .with_selected_signal(state.ctx.material.debug_normals.signal())
            .with_on_click(clone!(state => move || {
                state.ctx.material.debug_normals.set_neq(!state.ctx.material.debug_normals.get());
            }))
            .render()
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum FragmentShaderKind {
    Pbr,
    DebugNormals,
}
