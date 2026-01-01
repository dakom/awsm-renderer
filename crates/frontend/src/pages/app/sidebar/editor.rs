use crate::{
    atoms::checkbox::{Checkbox, CheckboxStyle},
    pages::app::context::AppContext,
    prelude::*,
};

pub struct SidebarEditor {
    ctx: AppContext,
}

impl SidebarEditor {
    pub fn new(ctx: AppContext) -> Arc<Self> {
        Arc::new(Self { ctx })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("margin-top", "1rem")
                .style("flex-direction", "column")
                .style("justify-content", "flex-start")
                .style("gap", "1rem")
            }
        });

        html!("div", {
            .class(&*CONTAINER)
            .child(state.render_show_grid())
            .child(state.render_show_gizmo_translation())
            .child(state.render_show_gizmo_rotation())
            .child(state.render_show_gizmo_scale())
        })
    }

    fn render_show_grid(self: &Arc<Self>) -> Dom {
        let state = self;

        Checkbox::new(CheckboxStyle::Dark)
            .with_content_after(html!("span", {
                .text("Show Grid")
            }))
            .with_selected_signal(state.ctx.editor_grid_enabled.signal())
            .with_on_click(clone!(state => move || {
                state.ctx.editor_grid_enabled.set_neq(!state.ctx.editor_grid_enabled.get());
            }))
            .render()
    }

    fn render_show_gizmo_translation(self: &Arc<Self>) -> Dom {
        let state = self;

        Checkbox::new(CheckboxStyle::Dark)
            .with_content_after(html!("span", {
                .text("Show Translation Gizmo")
            }))
            .with_selected_signal(state.ctx.editor_gizmo_translation_enabled.signal())
            .with_on_click(clone!(state => move || {
                state.ctx.editor_gizmo_translation_enabled.set_neq(!state.ctx.editor_gizmo_translation_enabled.get());
            }))
            .render()
    }

    fn render_show_gizmo_rotation(self: &Arc<Self>) -> Dom {
        let state = self;

        Checkbox::new(CheckboxStyle::Dark)
            .with_content_after(html!("span", {
                .text("Show Rotation Gizmo")
            }))
            .with_selected_signal(state.ctx.editor_gizmo_rotation_enabled.signal())
            .with_on_click(clone!(state => move || {
                state.ctx.editor_gizmo_rotation_enabled.set_neq(!state.ctx.editor_gizmo_rotation_enabled.get());
            }))
            .render()
    }

    fn render_show_gizmo_scale(self: &Arc<Self>) -> Dom {
        let state = self;

        Checkbox::new(CheckboxStyle::Dark)
            .with_content_after(html!("span", {
                .text("Show Scale Gizmo")
            }))
            .with_selected_signal(state.ctx.editor_gizmo_scale_enabled.signal())
            .with_on_click(clone!(state => move || {
                state.ctx.editor_gizmo_scale_enabled.set_neq(!state.ctx.editor_gizmo_scale_enabled.get());
            }))
            .render()
    }
}
