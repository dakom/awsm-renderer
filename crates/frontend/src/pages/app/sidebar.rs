use crate::prelude::*;

pub struct AppSidebar { 
}

impl AppSidebar {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("margin", "1rem")
            }
        });
        html!("div", {
            .class(&*CONTAINER)
            .child(html!("div", {
                .class([FontSize::H3.class(), ColorText::SidebarHeader.class()])
                .text("App Sidebar")
            }))
        })
    }
}