#[derive(Debug, Clone, Default)]
pub struct PrimitiveState {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createRenderPipeline#primitive
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuPrimitiveState.html
    pub cull_mode: Option<CullMode>,
    pub front_face: Option<FrontFace>,
    pub strip_index_format: Option<IndexFormat>,
    pub topology: Option<PrimitiveTopology>,
    pub unclipped_depth: Option<bool>,
}

pub type PrimitiveTopology = web_sys::GpuPrimitiveTopology;
pub type IndexFormat = web_sys::GpuIndexFormat;
pub type FrontFace = web_sys::GpuFrontFace;
pub type CullMode = web_sys::GpuCullMode;

impl PrimitiveState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cull_mode(mut self, cull_mode: CullMode) -> Self {
        self.cull_mode = Some(cull_mode);
        self
    }

    pub fn with_front_face(mut self, front_face: FrontFace) -> Self {
        self.front_face = Some(front_face);
        self
    }

    pub fn with_strip_index_format(mut self, strip_index_format: IndexFormat) -> Self {
        self.strip_index_format = Some(strip_index_format);
        self
    }

    pub fn with_topology(mut self, topology: PrimitiveTopology) -> Self {
        self.topology = Some(topology);
        self
    }

    pub fn with_unclipped_depth(mut self, unclipped_depth: bool) -> Self {
        self.unclipped_depth = Some(unclipped_depth);
        self
    }
}

impl From<PrimitiveState> for web_sys::GpuPrimitiveState {
    fn from(state: PrimitiveState) -> web_sys::GpuPrimitiveState {
        let state_js = web_sys::GpuPrimitiveState::new();
        if let Some(cull_mode) = state.cull_mode {
            state_js.set_cull_mode(cull_mode);
        }
        if let Some(front_face) = state.front_face {
            state_js.set_front_face(front_face);
        }
        if let Some(strip_index_format) = state.strip_index_format {
            state_js.set_strip_index_format(strip_index_format);
        }
        if let Some(topology) = state.topology {
            state_js.set_topology(topology);
        }

        if let Some(unclipped_depth) = state.unclipped_depth {
            state_js.set_unclipped_depth(unclipped_depth);
        }

        state_js
    }
}