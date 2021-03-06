use wasm_bindgen::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RenderMode {
    Shaded,
    DebugEntityPicker
}
impl From<RenderMode> for JsValue {
    fn from(render_mode:RenderMode) -> Self {
        match render_mode {
            RenderMode::Shaded => JsValue::from_str("shaded"),
            RenderMode::DebugEntityPicker=> JsValue::from_str("debug-entity-picker"),
        }
    }
}
impl From<String> for RenderMode {
    fn from(value:String) -> Self {
        match value.as_ref() {
            "shaded" => RenderMode::Shaded,
            "debug-entity-picker" => RenderMode::DebugEntityPicker,
            _ => unimplemented!("not a render mode: {}", value)
        }
    }
}


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SelectMode {
    Camera,
    Object 
}
impl From<SelectMode> for JsValue {
    fn from(select_mode:SelectMode) -> Self {
        match select_mode {
            SelectMode::Camera => JsValue::from_str("camera"),
            SelectMode::Object => JsValue::from_str("object"),
        }
    }
}
impl From<String> for SelectMode {
    fn from(value:String) -> Self {
        match value.as_ref() {
            "camera" => SelectMode::Camera,
            "object" => SelectMode::Object,
            _ => unimplemented!("not a select mode: {}", value)
        }
    }
}
