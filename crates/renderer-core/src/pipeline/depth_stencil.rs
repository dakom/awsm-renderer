use crate::texture::TextureFormat;

use crate::compare::CompareFunction;

#[derive(Debug, Clone)]
pub struct DepthStencilState {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#depthstencil_object_structure
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuDepthStencilState.html
    pub depth_bias: Option<i32>,
    pub depth_bias_clamp: Option<f32>,
    pub depth_bias_slope_scale: Option<f32>,
    pub depth_compare: Option<CompareFunction>,
    pub depth_write_enabled: Option<bool>,
    pub format: TextureFormat,
    pub stencil_back: Option<StencilFaceState>,
    pub stencil_front: Option<StencilFaceState>,
    pub stencil_read_mask: Option<u32>,
    pub stencil_write_mask: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct StencilFaceState {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#stencilback
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuStencilFaceState.html
    pub compare: Option<CompareFunction>,
    pub fail_op: Option<StencilOperation>,
    pub depth_fail_op: Option<StencilOperation>,
    pub pass_op: Option<StencilOperation>,
}

pub type StencilOperation = web_sys::GpuStencilOperation;

impl DepthStencilState {
    pub fn new(format: TextureFormat) -> Self {
        Self {
            depth_bias: None,
            depth_bias_clamp: None,
            depth_bias_slope_scale: None,
            depth_compare: None,
            depth_write_enabled: None,
            format,
            stencil_back: None,
            stencil_front: None,
            stencil_read_mask: None,
            stencil_write_mask: None,
        }
    }
}

impl From<DepthStencilState> for web_sys::GpuDepthStencilState {
    fn from(state: DepthStencilState) -> web_sys::GpuDepthStencilState {
        let state_js = web_sys::GpuDepthStencilState::new(state.format);
        if let Some(depth_bias) = state.depth_bias {
            state_js.set_depth_bias(depth_bias);
        }
        if let Some(depth_bias_clamp) = state.depth_bias_clamp {
            state_js.set_depth_bias_clamp(depth_bias_clamp);
        }
        if let Some(depth_bias_slope_scale) = state.depth_bias_slope_scale {
            state_js.set_depth_bias_slope_scale(depth_bias_slope_scale);
        }
        if let Some(depth_compare) = state.depth_compare {
            state_js.set_depth_compare(depth_compare);
        }
        if let Some(depth_write_enabled) = state.depth_write_enabled {
            state_js.set_depth_write_enabled(depth_write_enabled);
        }
        if let Some(stencil_back) = state.stencil_back {
            state_js.set_stencil_back(&web_sys::GpuStencilFaceState::from(stencil_back));
        }
        if let Some(stencil_front) = state.stencil_front {
            state_js.set_stencil_front(&web_sys::GpuStencilFaceState::from(stencil_front));
        }
        if let Some(stencil_read_mask) = state.stencil_read_mask {
            state_js.set_stencil_read_mask(stencil_read_mask);
        }
        if let Some(stencil_write_mask) = state.stencil_write_mask {
            state_js.set_stencil_write_mask(stencil_write_mask);
        }
        state_js
    }
}

impl From<StencilFaceState> for web_sys::GpuStencilFaceState {
    fn from(state: StencilFaceState) -> web_sys::GpuStencilFaceState {
        let state_js = web_sys::GpuStencilFaceState::new();

        if let Some(compare) = state.compare {
            state_js.set_compare(compare);
        }
        if let Some(fail_op) = state.fail_op {
            state_js.set_fail_op(fail_op);
        }
        if let Some(depth_fail_op) = state.depth_fail_op {
            state_js.set_depth_fail_op(depth_fail_op);
        }
        if let Some(pass_op) = state.pass_op {
            state_js.set_pass_op(pass_op);
        }

        state_js
    }
}
