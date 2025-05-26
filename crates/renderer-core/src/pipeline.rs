pub mod constants;
pub mod depth_stencil;
pub mod fragment;
pub mod layout;
pub mod multisample;
pub mod primitive;
pub mod vertex;

use std::collections::BTreeMap;

use constants::{ConstantOverrideKey, ConstantOverrideValue};
use depth_stencil::DepthStencilState;
use fragment::FragmentState;
use layout::PipelineLayoutKind;
use multisample::MultisampleState;
use primitive::PrimitiveState;
use vertex::VertexState;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone)]
pub struct RenderPipelineDescriptor<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#descriptor
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuRenderPipelineDescriptor.html
    // fill this out with a lot more detail
    depth_stencil: Option<DepthStencilState>,
    fragment: Option<FragmentState<'a>>,
    label: Option<&'a str>,
    layout: PipelineLayoutKind,
    multisample: Option<MultisampleState>,
    primitive: Option<PrimitiveState>,
    vertex: VertexState<'a>,
}

impl<'a> RenderPipelineDescriptor<'a> {
    pub fn new(vertex: VertexState<'a>, label: Option<&'a str>) -> Self {
        Self {
            depth_stencil: None,
            fragment: None,
            label,
            layout: PipelineLayoutKind::Auto,
            multisample: None,
            primitive: None,
            vertex,
        }
    }

    pub fn with_depth_stencil(mut self, depth_stencil: DepthStencilState) -> Self {
        self.depth_stencil = Some(depth_stencil);
        self
    }
    pub fn with_fragment(mut self, fragment: FragmentState<'a>) -> Self {
        self.fragment = Some(fragment);
        self
    }

    pub fn with_layout(mut self, layout: PipelineLayoutKind) -> Self {
        self.layout = layout;
        self
    }
    pub fn with_multisample(mut self, multisample: MultisampleState) -> Self {
        self.multisample = Some(multisample);
        self
    }
    pub fn with_primitive(mut self, primitive: PrimitiveState) -> Self {
        self.primitive = Some(primitive);
        self
    }
}

pub struct ComputePipelineDescriptor<'a, 'b> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createComputePipeline#descriptor
    label: Option<&'a str>,
    layout: PipelineLayoutKind,
    compute: ProgrammableStage<'b>,
}

impl<'a, 'b> ComputePipelineDescriptor<'a, 'b> {
    pub fn new(
        compute: ProgrammableStage<'b>,
        layout: PipelineLayoutKind,
        label: Option<&'a str>,
    ) -> Self {
        Self {
            label,
            layout,
            compute,
        }
    }
}

pub struct ProgrammableStage<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createComputePipeline#descriptor
    module: web_sys::GpuShaderModule,
    entry_point: Option<&'a str>,
    constants: BTreeMap<ConstantOverrideKey, ConstantOverrideValue>,
}

impl<'a> ProgrammableStage<'a> {
    pub fn new(module: web_sys::GpuShaderModule, entry_point: Option<&'a str>) -> Self {
        Self {
            module,
            entry_point,
            constants: BTreeMap::new(),
        }
    }

    pub fn with_push_constant(
        mut self,
        key: ConstantOverrideKey,
        value: ConstantOverrideValue,
    ) -> Self {
        self.constants.insert(key, value);
        self
    }
}

// js conversions
impl From<RenderPipelineDescriptor<'_>> for web_sys::GpuRenderPipelineDescriptor {
    fn from(pipeline: RenderPipelineDescriptor) -> web_sys::GpuRenderPipelineDescriptor {
        let RenderPipelineDescriptor {
            depth_stencil,
            fragment,
            label,
            layout,
            multisample,
            primitive,
            vertex,
        } = pipeline;

        let pipeline_js = web_sys::GpuRenderPipelineDescriptor::new(
            &layout.into(),
            &web_sys::GpuVertexState::from(vertex),
        );

        if let Some(depth_stencil) = depth_stencil {
            pipeline_js.set_depth_stencil(&web_sys::GpuDepthStencilState::from(depth_stencil));
        }

        if let Some(fragment) = fragment {
            pipeline_js.set_fragment(&web_sys::GpuFragmentState::from(fragment));
        }

        if let Some(multisample) = multisample {
            pipeline_js.set_multisample(&web_sys::GpuMultisampleState::from(multisample));
        }

        if let Some(primitive) = primitive {
            pipeline_js.set_primitive(&web_sys::GpuPrimitiveState::from(primitive));
        }

        if let Some(label) = label {
            pipeline_js.set_label(label.as_ref());
        }

        pipeline_js
    }
}

impl<'a, 'b> From<ComputePipelineDescriptor<'a, 'b>> for web_sys::GpuComputePipelineDescriptor {
    fn from(pipeline: ComputePipelineDescriptor) -> web_sys::GpuComputePipelineDescriptor {
        let ComputePipelineDescriptor {
            label,
            layout,
            compute,
        } = pipeline;

        let pipeline_js =
            web_sys::GpuComputePipelineDescriptor::new(&layout.into(), &compute.into());

        if let Some(label) = label {
            pipeline_js.set_label(label);
        }

        pipeline_js
    }
}

impl From<ProgrammableStage<'_>> for web_sys::GpuProgrammableStage {
    fn from(compute: ProgrammableStage) -> web_sys::GpuProgrammableStage {
        let compute_js = web_sys::GpuProgrammableStage::new(&compute.module);

        if let Some(entry_point) = compute.entry_point {
            compute_js.set_entry_point(entry_point);
        }

        if !compute.constants.is_empty() {
            let obj = js_sys::Object::new();
            for (binding, constant) in compute.constants {
                js_sys::Reflect::set(&obj, &JsValue::from(binding), &JsValue::from(constant))
                    .unwrap_throw();
            }
            compute_js.set_constants(&obj);
        }

        compute_js
    }
}
