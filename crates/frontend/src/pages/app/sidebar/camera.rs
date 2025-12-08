use crate::{
    pages::app::{context::AppContext, scene::camera::CameraId},
    prelude::*,
};

use super::render_dropdown_label;

pub struct SidebarCamera {
    ctx: AppContext,
}

impl SidebarCamera {
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
            .child(state.render_camera_selector())
        })
    }

    fn render_camera_selector(self: &Arc<Self>) -> Dom {
        let state = self;

        render_dropdown_label(
            "Projection",
            Dropdown::new()
                .with_intial_selected(Some(state.ctx.camera_id.get()))
                .with_bg_color(ColorBackground::Dropdown)
                .with_on_change(clone!(state => move |id| {
                    state.ctx.camera_id.set_neq(*id);
                }))
                .with_options([
                    ("Orthographic".to_string(), CameraId::Orthographic),
                    ("Perspective".to_string(), CameraId::Perspective),
                ])
                .render(),
        )
    }
}
