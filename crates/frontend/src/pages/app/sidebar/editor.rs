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
            .child(state.render_show_gizmos())
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

    fn render_show_gizmos(self: &Arc<Self>) -> Dom {
        let state = self;

        Checkbox::new(CheckboxStyle::Dark)
            .with_content_after(html!("span", {
                .text("Show Gizmos")
            }))
            .with_selected_signal(state.ctx.editor_gizmos_enabled.signal())
            .with_on_click(clone!(state => move || {
                state.ctx.editor_gizmos_enabled.set_neq(!state.ctx.editor_gizmos_enabled.get());
            }))
            .render()
    }
}
