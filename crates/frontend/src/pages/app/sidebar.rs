mod camera;
mod gltf;
pub mod material;
mod post_processing;
mod textures;

use camera::SidebarCamera;
use gltf::SidebarGltf;
use material::SidebarMaterial;

use crate::{
    models::collections::{GltfId, GLTF_SETS},
    pages::app::sidebar::{post_processing::SidebarPostProcessing, textures::SidebarTextures},
    prelude::*,
};

use super::context::AppContext;

pub struct AppSidebar {
    section: Mutable<Option<SidebarSection>>,
    ctx: AppContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SidebarSection {
    Gltf,
    Material,
    Animation,
    Lighting,
    PostProcessing,
    Camera,
    Textures,
}

impl AppSidebar {
    pub fn new(ctx: AppContext) -> Arc<Self> {
        Arc::new(Self {
            ctx,
            section: Mutable::new(CONFIG.initial_sidebar_open),
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
                self.render_section(SidebarSection::Gltf),
                self.render_section(SidebarSection::Material),
                self.render_section(SidebarSection::Animation),
                self.render_section(SidebarSection::Lighting),
                self.render_section(SidebarSection::PostProcessing),
                self.render_section(SidebarSection::Camera),
                self.render_section(SidebarSection::Textures),
            ])
        })
    }

    fn render_section(self: &Arc<Self>, section: SidebarSection) -> Dom {
        let state = self;
        html!("div", {
            .child(state.render_section_header(section))
            .child_signal(state.section.signal().map(clone!(state => move |current| {
                match current {
                    None => None,
                    Some(current) => {
                        if current == section {
                            Some(html!("div", {
                                .style("margin-left", "1rem")
                                .style("margin-bottom", "1rem")
                                .child(match section {
                                    SidebarSection::Gltf => SidebarGltf::new().render(),
                                    SidebarSection::Material => SidebarMaterial::new(state.ctx.clone()).render(),
                                    SidebarSection::Animation => html!("div", {
                                        .class([FontSize::Lg.class(), ColorText::SidebarHeader.class()])
                                        .text("TODO")
                                    }),
                                    SidebarSection::Lighting => html!("div", {
                                        .class([FontSize::Lg.class(), ColorText::SidebarHeader.class()])
                                        .text("TODO")
                                    }),
                                    SidebarSection::PostProcessing => SidebarPostProcessing::new(state.ctx.clone()).render(),
                                    SidebarSection::Camera => SidebarCamera::new(state.ctx.clone()).render(),
                                    SidebarSection::Textures => SidebarTextures::new(state.ctx.clone()).render(),
                                })
                            }))
                        } else {
                            None
                        }
                    }
                }
            })))
        })
    }

    fn render_section_header(self: &Arc<Self>, section: SidebarSection) -> Dom {
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
                let current = state.section.get();
                if current == Some(section) {
                    state.section.set(None);
                } else {
                    state.section.set_neq(Some(section));
                }
            })))
            .class(&*MENU_ITEM)
            .child(
                svg!("svg", {
                    .class(&*MENU_ITEM_SVG)
                    .attrs!{
                        "xmlns": "http://www.w3.org/2000/svg",
                        "viewBox": "0 0 24 24",
                    }
                    .children(
                        match section {
                            SidebarSection::Gltf => {
                                vec![svg!("path", {
                                    .attr("d", "M12 2l8 4.5v9L12 22l-8-6.5v-9L12 2zm0 2.2L6 7v8l6 4.8 6-4.8V7l-6-2.8z")
                                })]
                            },
                            SidebarSection::Material => {
                                vec![svg!("path", {
                                    .attr("d", "M12 2 L4 7 L12 12 L20 7 Z M4 7 L4 17 L12 22 L12 12 Z M12 12 L12 22 L20 17 L20 7 Z")
                                })]
                            },
                            SidebarSection::Animation => {
                                vec![svg!("path", {
                                    .attr("d", "M8 5v14l11-7-11-7z")
                                })]
                            },
                            SidebarSection::Lighting => {
                                vec![svg!("path", {
                                    .attr("d", "M9 21h6v-1H9v1zm3-19a7 7 0 00-4 12.6V17a1 1 0 001 1h6a1 1 0 001-1v-2.4A7 7 0 0012 2zm3 12.7V16h-6v-1.3a5 5 0 116 0z")
                                })]
                            },
                            SidebarSection::PostProcessing=> {
                                vec![svg!("path", {
                                    .attr("d", "M12 2a10 10 0 100 20 10 10 0 000-20zm0 18a8 8 0 110-16 8 8 0 010 16zm0-14a6 6 0 100 12 6 6 0 000-12z")
                                })]
                            },
                            SidebarSection::Camera => {
                                vec![
                                    svg!("circle", {
                                        .attrs!{ "cx": "7", "cy": "7", "r": "3",}
                                    }),
                                    svg!("circle", {
                                        .attrs!{ "cx": "15", "cy": "7", "r": "3",}
                                    }),
                                    svg!("rect", {
                                        .attrs!{ "x": "3", "y": "10", "width": "12", "height": "8", "rx": "1.2",}
                                    }),
                                    svg!("polygon", {
                                        .attr("points", "15 12 22 9 22 19 15 16")
                                    }),
                                ]
                            },
                            SidebarSection::Textures => {
                                vec![
                                    svg!("rect", {
                                        .attrs!{
                                            "x": "2", "y": "2", "width": "8", "height": "8",
                                            "fill": "currentColor"
                                        }
                                    }),
                                    svg!("rect", {
                                        .attrs!{
                                            "x": "10", "y": "2", "width": "8", "height": "8",
                                            "fill": "none", "stroke": "currentColor", "stroke-width": "1"
                                        }
                                    }),
                                    svg!("rect", {
                                        .attrs!{
                                            "x": "2", "y": "10", "width": "8", "height": "8",
                                            "fill": "none", "stroke": "currentColor", "stroke-width": "1"
                                        }
                                    }),
                                    svg!("rect", {
                                        .attrs!{
                                            "x": "10", "y": "10", "width": "8", "height": "8",
                                            "fill": "currentColor"
                                        }
                                    }),
                                ]
                            },
                        }
                    )
                })
            )
            .child(
                html!("span", {
                    .text(match section {
                        SidebarSection::Gltf => "Pick GLTF Model",
                        SidebarSection::Material => "Material",
                        SidebarSection::Animation => "Animation",
                        SidebarSection::Lighting => "Lighting",
                        SidebarSection::PostProcessing => "Post Processing",
                        SidebarSection::Camera => "Camera",
                        SidebarSection::Textures => "Textures",
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

pub fn render_dropdown_label(label: &str, dropdown: Dom) -> Dom {
    static CONTAINER: LazyLock<String> = LazyLock::new(|| {
        class! {
            .style("display", "flex")
            .style("flex-direction", "column")
            .style("margin", "1rem")
            .style("gap", "1rem")
        }
    });

    html!("div", {
        .class(&*CONTAINER)
        .child(html!("div", {
            .class([FontSize::Xlg.class(), ColorText::SidebarHeader.class()])
            .text(label)
        }))
        .child(dropdown)
    })
}

pub fn render_checkbox_label(label: &str, checkbox: Dom) -> Dom {
    static CONTAINER: LazyLock<String> = LazyLock::new(|| {
        class! {
            .style("display", "flex")
            .style("flex-direction", "column")
            .style("margin", "1rem")
            .style("gap", "1rem")
        }
    });

    html!("div", {
        .class(&*CONTAINER)
        .child(html!("div", {
            .class([FontSize::Xlg.class(), ColorText::SidebarHeader.class()])
            .text(label)
        }))
        .child(checkbox)
    })
}
