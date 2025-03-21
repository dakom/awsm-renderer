use wasm_bindgen::prelude::*;

#[derive(Clone, Debug)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn new_values(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn new_slice(arr: &[f64]) -> Self {
        if arr.len() != 4 {
            panic!("Array length must be 4");
        }
        Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        }
    }

    pub fn as_js_value(&self) -> JsValue {
        let arr = js_sys::Array::new();
        arr.push(&JsValue::from_f64(self.r));
        arr.push(&JsValue::from_f64(self.g));
        arr.push(&JsValue::from_f64(self.b));
        arr.push(&JsValue::from_f64(self.a));

        arr.into()
    }
}