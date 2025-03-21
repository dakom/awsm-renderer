use wasm_bindgen::prelude::*;

// https://gpuweb.github.io/gpuweb/#abstract-opdef-to-wgsl-type
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ConstantOverride {
    Bool(bool),
    I32(i32),
    U32(u32),
    F32(f32),
    //F16(f16),
}

impl From<ConstantOverride> for JsValue {
    fn from(constant: ConstantOverride) -> Self {
        match constant {
            ConstantOverride::Bool(value) => JsValue::from_bool(value),
            ConstantOverride::I32(value) => JsValue::from_f64(value as f64),
            ConstantOverride::U32(value) => JsValue::from_f64(value as f64),
            ConstantOverride::F32(value) => JsValue::from_f64(value as f64),
        }
    }
}