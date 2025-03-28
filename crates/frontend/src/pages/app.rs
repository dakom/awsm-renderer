mod content;
mod renderer;
mod sidebar;

use content::AppContent;
use renderer::AppRenderer;
use sidebar::AppSidebar;

use crate::prelude::*;

pub struct AppUi {
    renderer: AppRenderer,
}

impl AppUi {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            renderer: AppRenderer::new(),
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "row")  // changed to row for horizontal layout
                .style("width", "100%")
                .style("height", "100vh")        // full viewport height
                .style("overflow", "hidden")     // prevent window-level scrolling
                .style("position", "fixed")      // ensure it stays fixed to viewport
                .style("top", "0")
                .style("left", "0")
                .style("margin", "0")            // remove the top margin
            }
        });
        html!("div", {
            .class(&*CONTAINER)
            .child(html!("div", {  // left column
                .style("flex", "0 0 auto")  // don't grow, don't shrink, size to content
                .style("overflow-y", "auto")  // scroll if content overflows
                .style("height", "100%")
                .class(ColorBackground::Sidebar.class())
                .child(AppSidebar::new(self.renderer.clone()).render())
            }))
            .child(html!("div", {  // right column
                .style("flex", "1")  // grow to fill remaining space
                .style("overflow-y", "auto")  // scroll if content overflows
                .style("height", "100%")
                .class(ColorBackground::GltfContent.class())
                .child(AppContent::new(self.renderer.clone()).render())
            }))
        })
    }
}
