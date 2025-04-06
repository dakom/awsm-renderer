use wasm_bindgen::prelude::*;

use super::constants::ConstantOverride;

#[derive(Debug, Clone)]
pub struct VertexState<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#vertex_object_structure
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuVertexState.html
    pub constants: Vec<(u16, ConstantOverride)>,
    pub entry_point: Option<&'a str>,
    pub module: &'a web_sys::GpuShaderModule,
    pub buffers: Vec<VertexBufferLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VertexBufferLayout {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#buffers
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuVertexBufferLayout.html
    pub array_stride: u64,
    pub attributes: Vec<VertexAttribute>,
    pub step_mode: Option<VertexStepMode>,
}

impl std::hash::Hash for VertexBufferLayout {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.array_stride.hash(state);
        self.step_mode.map(|x| x as u32).hash(state);
        self.attributes.hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexAttribute {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#attributes
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuVertexAttribute.html
    pub format: VertexFormat,
    pub offset: u64,
    pub shader_location: u32,
}

impl std::hash::Hash for VertexAttribute {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.format as u32).hash(state);
        self.offset.hash(state);
        self.shader_location.hash(state);
    }
}

// https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#stepmode
pub type VertexStepMode = web_sys::GpuVertexStepMode;

// https://rustwasm.github.io/wasm-bindgen/api/web_sys/enum.GpuVertexFormat.html
pub type VertexFormat = web_sys::GpuVertexFormat;

// JS conversion

impl<'a> VertexState<'a> {
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
                js_sys::Reflect::set(&obj, &JsValue::from(*binding), &JsValue::from(*constant))
                    .unwrap_throw();
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
                attributes.push(&web_sys::GpuVertexAttribute::from(*attribute));
            }
        }

        let buffer_layout_js =
            web_sys::GpuVertexBufferLayout::new(buffer_layout.array_stride as f64, &attributes);

        if let Some(step_mode) = buffer_layout.step_mode {
            buffer_layout_js.set_step_mode(step_mode);
        }

        buffer_layout_js
    }
}

impl From<VertexAttribute> for web_sys::GpuVertexAttribute {
    fn from(attribute: VertexAttribute) -> web_sys::GpuVertexAttribute {
        web_sys::GpuVertexAttribute::new(
            attribute.format,
            attribute.offset as f64,
            attribute.shader_location,
        )
    }
}
