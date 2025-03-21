use wasm_bindgen::prelude::*;

use super::constants::ConstantOverride;

#[derive(Debug, Clone)]
pub struct VertexState <'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#vertex_object_structure
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuVertexState.html
    pub constants: Vec<(u16, ConstantOverride)>,
    pub entry_point: Option<&'a str>,
    pub module: &'a web_sys::GpuShaderModule,
    pub buffers: Vec<VertexBufferLayout>,
}

#[derive(Debug, Clone)]
pub struct VertexBufferLayout {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#buffers
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuVertexBufferLayout.html
    pub array_stride: f64,
    pub attributes: Vec<VertexAttribute>,
    pub step_mode: Option<VertexStepMode>,
}


// https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#attributes
pub type VertexAttribute = web_sys::GpuVertexAttribute;

// https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#stepmode
pub type VertexStepMode = web_sys::GpuVertexStepMode;

pub type VertexFormat = web_sys::GpuVertexFormat;

// JS conversion

impl <'a> VertexState <'a> {
    pub fn new(module: &'a web_sys::GpuShaderModule, entry_point: Option<&'a str>) -> Self {
        Self {
            constants: Vec::new(),
            entry_point,
            module,
            buffers: Vec::new(),
        }
    }
}

impl From<VertexState<'_>> for web_sys::GpuVertexState {
    fn from(state: VertexState) -> Self {
        let state_js = web_sys::GpuVertexState::new(state.module);

        if let Some(entry_point) = &state.entry_point {
            state_js.set_entry_point(entry_point);
        }

        if !state.constants.is_empty() {
            let obj = js_sys::Object::new();
            for (binding, constant) in &state.constants {
                js_sys::Reflect::set(&obj, &JsValue::from(*binding), &JsValue::from(*constant)).unwrap_throw();
            }
            state_js.set_constants(&obj);
        }

        if !state.buffers.is_empty() {
            let buffers = js_sys::Array::new();
            for buffer in state.buffers {
                buffers.push(&web_sys::GpuVertexBufferLayout::from(buffer));
            }
            state_js.set_buffers(&buffers);
        }

        state_js
    }
}

impl From<VertexBufferLayout> for web_sys::GpuVertexBufferLayout {
    fn from(buffer_layout: VertexBufferLayout) -> web_sys::GpuVertexBufferLayout {

        let attributes = js_sys::Array::new();
        if !buffer_layout.attributes.is_empty() {
            for attribute in &buffer_layout.attributes {
                attributes.push(attribute);
            }
        }

        let buffer_layout_js = web_sys::GpuVertexBufferLayout::new(buffer_layout.array_stride, &attributes);

        if let Some(step_mode) = buffer_layout.step_mode {
            buffer_layout_js.set_step_mode(step_mode);
        }

        buffer_layout_js
    }
}