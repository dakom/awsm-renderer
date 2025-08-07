use wasm_bindgen::prelude::*;

#[derive(Clone, Debug)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub const MID_GREY: Self = Self {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    };

    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    pub const RED_U32: Self = Self {
        r: u32::MAX as f64,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

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

    pub fn perceptual_to_linear(self) -> Self {
        Self {
            r: perceptual_to_linear(self.r),
            g: perceptual_to_linear(self.g),
            b: perceptual_to_linear(self.b),
            a: self.a,
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

fn perceptual_to_linear(perceptual: f64) -> f64 {
    // Same implementation as srgb_to_linear
    if perceptual <= 0.04045 {
        perceptual / 12.92
    } else {
        ((perceptual + 0.055) / 1.055).powf(2.4)
    }
}
