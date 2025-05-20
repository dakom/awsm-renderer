use std::collections::BTreeMap;

use crate::texture::TextureFormat;
use wasm_bindgen::prelude::*;

use super::constants::{ConstantOverrideKey, ConstantOverrideValue};

#[derive(Debug, Clone)]
pub struct FragmentState<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#fragment_object_structure
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuFragmentState.html
    pub constants: BTreeMap<ConstantOverrideKey, ConstantOverrideValue>,
    pub entry_point: Option<&'a str>,
    pub module: &'a web_sys::GpuShaderModule,
    pub targets: Vec<ColorTargetState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorTargetState {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#targets
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuColorTargetState.html
    pub blend: Option<BlendState>,
    pub format: TextureFormat,
    pub write_mask: Option<u32>,
}

impl std::hash::Hash for ColorTargetState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.blend.hash(state);
        (self.format as u32).hash(state);
        self.write_mask.hash(state);
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct BlendState {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#blend
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuBlendState.html
    pub alpha: BlendComponent,
    pub color: BlendComponent,
}

impl BlendState {
    pub fn new(color: BlendComponent, alpha: BlendComponent) -> Self {
        Self { color, alpha }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlendComponent {
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuBlendComponent.html
    pub operation: Option<BlendOperation>,
    pub src_factor: Option<BlendFactor>,
    pub dst_factor: Option<BlendFactor>,
}

impl BlendComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_operation(mut self, operation: BlendOperation) -> Self {
        self.operation = Some(operation);
        self
    }
    pub fn with_src_factor(mut self, src_factor: BlendFactor) -> Self {
        self.src_factor = Some(src_factor);
        self
    }
    pub fn with_dst_factor(mut self, dst_factor: BlendFactor) -> Self {
        self.dst_factor = Some(dst_factor);
        self
    }
}

impl std::hash::Hash for BlendComponent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.operation.map(|x| x as u32).hash(state);
        self.src_factor.map(|x| x as u32).hash(state);
        self.dst_factor.map(|x| x as u32).hash(state);
    }
}

// https://rustwasm.github.io/wasm-bindgen/api/web_sys/enum.GpuBlendFactor.html
pub type BlendFactor = web_sys::GpuBlendFactor;
// https://rustwasm.github.io/wasm-bindgen/api/web_sys/enum.GpuBlendOperation.html
pub type BlendOperation = web_sys::GpuBlendOperation;

// js conversions

impl<'a> FragmentState<'a> {
    pub fn new(
        module: &'a web_sys::GpuShaderModule,
        entry_point: Option<&'a str>,
        targets: Vec<ColorTargetState>,
    ) -> Self {
        Self {
            constants: BTreeMap::new(),
            entry_point,
            module,
            targets,
        }
    }

    pub fn with_constant(
        mut self,
        binding: ConstantOverrideKey,
        constant: ConstantOverrideValue,
    ) -> Self {
        self.constants.insert(binding, constant);
        self
    }
    pub fn with_target(mut self, target: ColorTargetState) -> Self {
        self.targets.push(target);
        self
    }
}

impl From<FragmentState<'_>> for web_sys::GpuFragmentState {
    fn from(state: FragmentState) -> web_sys::GpuFragmentState {
        let targets = js_sys::Array::new();
        for target in state.targets {
            targets.push(&web_sys::GpuColorTargetState::from(target));
        }

        let state_js = web_sys::GpuFragmentState::new(state.module, &targets);

        if let Some(entry_point) = state.entry_point {
            state_js.set_entry_point(entry_point);
        }

        if !state.constants.is_empty() {
            let obj = js_sys::Object::new();
            for (binding, constant) in state.constants {
                js_sys::Reflect::set(&obj, &JsValue::from(binding), &JsValue::from(constant))
                    .unwrap_throw();
            }
            state_js.set_constants(&obj);
        }

        state_js
    }
}

impl ColorTargetState {
    pub fn new(format: TextureFormat) -> Self {
        Self {
            blend: None,
            format,
            write_mask: None,
        }
    }
}

impl From<ColorTargetState> for web_sys::GpuColorTargetState {
    fn from(state: ColorTargetState) -> web_sys::GpuColorTargetState {
        let state_js = web_sys::GpuColorTargetState::new(state.format);

        if let Some(blend) = state.blend {
            state_js.set_blend(&web_sys::GpuBlendState::from(blend));
        }

        if let Some(write_mask) = state.write_mask {
            state_js.set_write_mask(write_mask);
        }

        state_js
    }
}

impl From<BlendState> for web_sys::GpuBlendState {
    fn from(state: BlendState) -> web_sys::GpuBlendState {
        web_sys::GpuBlendState::new(
            // not sure why these are reversed, but they are:
            // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuBlendState.html#method.new
            &web_sys::GpuBlendComponent::from(state.alpha),
            &web_sys::GpuBlendComponent::from(state.color),
        )
    }
}

impl From<BlendComponent> for web_sys::GpuBlendComponent {
    fn from(component: BlendComponent) -> web_sys::GpuBlendComponent {
        let component_js = web_sys::GpuBlendComponent::new();

        if let Some(operation) = component.operation {
            component_js.set_operation(operation);
        }
        if let Some(src_factor) = component.src_factor {
            component_js.set_src_factor(src_factor);
        }
        if let Some(dst_factor) = component.dst_factor {
            component_js.set_dst_factor(dst_factor);
        }

        component_js
    }
}
