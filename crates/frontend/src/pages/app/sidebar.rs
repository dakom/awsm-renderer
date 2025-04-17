mod gltf;

use gltf::SidebarGltf;

use crate::{
    models::collections::{GltfId, GLTF_SETS},
    prelude::*,
};

pub struct AppSidebar {
    section: Mutable<Section>
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Section {
    Gltf,
    Animation,
    Lighting,
    Environment
}

impl AppSidebar {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            section: Mutable::new(Section::Gltf)
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;

        static SIDEBAR: LazyLock<String> = LazyLock::new(|| {
            class! {

                .style("display", "flex")
                .style("flex-direction", "column")
                .style("margin-top", "1rem")
            }
        });

        html!("div", {
            .class([&*SIDEBAR, &*USER_SELECT_NONE])
            .children([
                self.render_section(Section::Gltf),
                self.render_section(Section::Animation),
                self.render_section(Section::Lighting),
                self.render_section(Section::Environment),
            ])
        })
    }

    fn render_section(self: &Arc<Self>, section: Section) -> Dom {
        let state = self;
        html!("div", {
            .child(state.render_section_header(section))
            .child_signal(state.section.signal().map(move |current| {
                if current == section {
                    Some(html!("div", {
                        .style("margin-left", "1rem")
                        .style("margin-bottom", "1rem")
                        .child(match section {
                            Section::Gltf => SidebarGltf::new().render(),
                            Section::Animation => html!("div", { 
                                .class([FontSize::Lg.class(), ColorText::SidebarHeader.class()])
                                .text("TODO") 
                            }),
                            Section::Lighting => html!("div", { 
                                .class([FontSize::Lg.class(), ColorText::SidebarHeader.class()])
                                .text("TODO") 
                            }),
                            Section::Environment => html!("div", { 
                                .class([FontSize::Lg.class(), ColorText::SidebarHeader.class()])
                                .text("TODO") 
                            }),
                        })
                    }))
                } else {
                    None
                }
            }))
        })
    }

    fn render_section_header(self: &Arc<Self>, section: Section) -> Dom {
        let state = self;

        static MENU_ITEM: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("font-size", FontSize::Lg.value())
                .style("color", ColorText::SidebarHeader.value())
                .style("display", "flex")
                .style("align-items", "center")
                .style("padding", "0.75rem 1rem")
                .style("gap", "0.6rem")
                .style("cursor", "pointer")
                .style("transition", "background-color 0.3s")
                .pseudo!(":hover", {
                    .style("background-color", "#636e72")
                    .style("color", "#ffffff")
                })
            }
        });

        static MENU_ITEM_SVG: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("width", FontSize::Lg.value())
                .style("height", FontSize::Lg.value())
                .style("fill", ColorText::SidebarHeader.value())
                .style("flex-shrink", "0")
            }
        });

        html!("div", {
            .apply(handle_on_click(clone!(state => move || {
                state.section.set_neq(section);
            })))
            .class(&*MENU_ITEM)
            .child(
                svg!("svg", {
                    .class(&*MENU_ITEM_SVG)
                    .attrs!{
                        "xmlns": "http://www.w3.org/2000/svg",
                        "viewBox": "0 0 24 24",
                    }
                    .child(
                        match section {
                            Section::Gltf => {
                                svg!("path", {
                                    .attr("d", "M12 2l8 4.5v9L12 22l-8-6.5v-9L12 2zm0 2.2L6 7v8l6 4.8 6-4.8V7l-6-2.8z")
                                })
                            },
                            Section::Animation => {
                                svg!("path", {
                                    .attr("d", "M8 5v14l11-7-11-7z")
                                })
                            },
                            Section::Lighting => {
                                svg!("path", {
                                    .attr("d", "M9 21h6v-1H9v1zm3-19a7 7 0 00-4 12.6V17a1 1 0 001 1h6a1 1 0 001-1v-2.4A7 7 0 0012 2zm3 12.7V16h-6v-1.3a5 5 0 116 0z")
                                })
                            },
                            Section::Environment => {
                                svg!("path", {
                                    .attr("d", "M12 2a10 10 0 100 20 10 10 0 000-20zm0 18a8 8 0 110-16 8 8 0 010 16zm0-14a6 6 0 100 12 6 6 0 000-12z")
                                })
                            },
                        }
                    )
                })
            )
            .child(
                html!("span", {
                    .text(match section {
                        Section::Gltf => "Pick GLTF Model",
                        Section::Animation => "Animation Settings",
                        Section::Lighting => "Lighting Settings",
                        Section::Environment => "Environment Settings",
                    })
                })
            )
        })
    }

}


pub fn current_model_signal() -> impl Signal<Item = Option<GltfId>> {
    Route::signal().map(|route| match route {
        Route::App(AppRoute::Model(model_id)) => Some(model_id),
        _ => None,
    })
}