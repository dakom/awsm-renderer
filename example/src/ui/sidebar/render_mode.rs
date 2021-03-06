use dominator::{html, Dom, events, clone, with_node};
use std::rc::Rc;
use crate::ui::state::*;
use super::Sidebar;
use crate::types::*;
use wasm_bindgen::prelude::*;
use web_sys::HtmlSelectElement;
use futures_signals::signal::SignalExt;

pub struct RenderModeDom {
}

impl RenderModeDom {
    pub fn render(sidebar:Rc<Sidebar>) -> Dom {
        let state = sidebar.state.clone();
        html!("select" => HtmlSelectElement, {
            .property_signal("value", state.render_mode.signal().map(JsValue::from))
            .with_node!(select => {
                .event(clone!(state => move |_:events::Change| {
                    let render_mode:RenderMode = select.value().into();
                    state.render_mode.set_neq(render_mode);
                }))
            })
            .children(vec![
                option("shaded", RenderMode::Shaded, state.clone()),
                option("entity picker debug", RenderMode::DebugEntityPicker, state.clone()),
            ])
        })
    }
}

fn option(label:&str, render_mode: RenderMode, _state:Rc<State>) -> Dom {
    html!("option", {
        .property("value", JsValue::from(render_mode))
        .text(label)
    })
}
