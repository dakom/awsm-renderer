use wasm_bindgen::prelude::*;

#[derive(Debug, Clone)]
pub enum PipelineLayoutKind {
    Auto,
    Custom(web_sys::GpuPipelineLayout)
}

impl Default for PipelineLayoutKind {
    fn default() -> Self {
        PipelineLayoutKind::Auto
    }
}

impl From<PipelineLayoutKind> for JsValue {
    fn from(layout_kind: PipelineLayoutKind) -> JsValue {
        match layout_kind {
            PipelineLayoutKind::Auto => JsValue::from_str("auto"),
            PipelineLayoutKind::Custom(layout) => layout.into(),
        }
    }
}


#[derive(Debug, Clone)]
pub struct PipelineLayoutDescriptor<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createPipelineLayout
    pub label: Option<&'a str>,
    pub bind_group_layouts: Vec<web_sys::GpuBindGroupLayout>,
}

impl <'a> PipelineLayoutDescriptor <'a> {
    pub fn new(label: Option<&'a str>) -> Self {
        Self {
            label,
            bind_group_layouts: Vec::new(),
        }
    }
}

impl From<PipelineLayoutDescriptor<'_>> for web_sys::GpuPipelineLayoutDescriptor {
    fn from(layout: PipelineLayoutDescriptor) -> Self {
        let bind_group_layouts = js_sys::Array::new();

        for bind_group_layout in layout.bind_group_layouts {
            bind_group_layouts.push(&bind_group_layout);
        }

        let layout_js = web_sys::GpuPipelineLayoutDescriptor::new(&bind_group_layouts);

        if let Some(label) = layout.label {
            layout_js.set_label(label);
        }

        layout_js
    }
}