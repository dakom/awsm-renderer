mod canvas;
mod scene;
mod sidebar;

use canvas::AppCanvas;
use sidebar::AppSidebar;

use crate::{models::collections::GltfId, prelude::*};

pub struct AppUi {}

impl AppUi {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "row")  // changed to row for horizontal layout
                .style("width", "100vw")         // full viewport
                .style("height", "100vh")
            }
        });
        html!("div", {
            .class(&*CONTAINER)
            .child(html!("div", {  // left column
                .style("flex", "0 0 auto")  // don't grow, don't shrink, size to content
                .style("overflow-y", "auto")  // scroll if content overflows
                .style("height", "100%")
                .class(ColorBackground::Sidebar.class())
                .child(AppSidebar::new().render())
            }))
            .child(html!("div", {  // right column
                .style("flex", "1")  // grow to fill remaining space
                .style("overflow-y", "auto")  // scroll if content overflows
                .style("height", "100%")
                .class(ColorBackground::GltfContent.class())
                .child(AppCanvas::new().render())
            }))
        })
    }
}
