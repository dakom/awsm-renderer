pub mod constants;
pub mod depth_stencil;
pub mod fragment;
pub mod layout;
pub mod multisample;
pub mod primitive;
pub mod vertex;

use depth_stencil::DepthStencilState;
use fragment::FragmentState;
use layout::PipelineLayoutKind;
use multisample::MultisampleState;
use primitive::PrimitiveState;
use vertex::VertexState;

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
