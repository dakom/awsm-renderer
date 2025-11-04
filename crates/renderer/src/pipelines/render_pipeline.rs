use std::collections::{BTreeMap, HashMap};

use awsm_renderer_core::{
    error::AwsmCoreError,
    pipeline::{
        constants::{ConstantOverrideKey, ConstantOverrideValue},
        depth_stencil::DepthStencilState,
        fragment::{ColorTargetState, FragmentState},
        layout::PipelineLayoutKind,
        multisample::MultisampleState,
        primitive::PrimitiveState,
        vertex::{VertexBufferLayout, VertexState},
        RenderPipelineDescriptor,
    },
    renderer::AwsmRendererWebGpu,
};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::AwsmBindGroupError,
    pipeline_layouts::{AwsmPipelineLayoutError, PipelineLayoutKey, PipelineLayouts},
    shaders::{ShaderKey, Shaders},
};

pub struct RenderPipelines {
    lookup: SlotMap<RenderPipelineKey, web_sys::GpuRenderPipeline>,
    cache: HashMap<RenderPipelineCacheKey, RenderPipelineKey>,
}

impl RenderPipelines {
    pub fn new() -> Self {
        Self {
            lookup: SlotMap::with_key(),
            cache: HashMap::new(),
        }
    }

    pub async fn get_key(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        shaders: &Shaders,
        pipeline_layouts: &PipelineLayouts,
        cache_key: RenderPipelineCacheKey,
    ) -> Result<RenderPipelineKey> {
        if let Some(key) = self.cache.get(&cache_key) {
            return Ok(*key);
        }

        let cache_key_clone = cache_key.clone();

        let shader_module = shaders
            .get(cache_key.shader_key)
            .ok_or(AwsmRenderPipelineError::MissingShader(cache_key.shader_key))?;

        let layout = pipeline_layouts.get(cache_key.layout_key)?;

        let mut vertex = VertexState::new(shader_module, None);
        vertex.buffer_layouts = cache_key.vertex_buffer_layouts;
        vertex.constants = cache_key.vertex_constants;

        let fragment = FragmentState::new(shader_module, None, cache_key.fragment_targets.clone());

        let mut descriptor = RenderPipelineDescriptor::new(vertex, None)
            .with_primitive(cache_key.primitive)
            .with_layout(PipelineLayoutKind::Custom(&layout))
            .with_fragment(fragment);

        if let Some(depth_stencil) = cache_key.depth_stencil {
            descriptor = descriptor.with_depth_stencil(depth_stencil);
        }

        if let Some(multisample) = cache_key.multisample {
            descriptor = descriptor.with_multisample(multisample);
        }

        let pipeline = gpu.create_render_pipeline(&descriptor.into()).await?;

        let key = self.lookup.insert(pipeline);
        self.cache.insert(cache_key_clone, key);
        Ok(key)
    }

    pub fn get(&self, key: RenderPipelineKey) -> Result<&web_sys::GpuRenderPipeline> {
        self.lookup
            .get(key)
            .ok_or(AwsmRenderPipelineError::NotFound(key))
    }
}

impl Default for RenderPipelines {
    fn default() -> Self {
        Self::new()
    }
}

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct RenderPipelineCacheKey {
    pub shader_key: ShaderKey,
    pub layout_key: PipelineLayoutKey,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub fragment_targets: Vec<ColorTargetState>,
    pub vertex_buffer_layouts: Vec<VertexBufferLayout>,
    pub vertex_constants: BTreeMap<ConstantOverrideKey, ConstantOverrideValue>,
    pub multisample: Option<MultisampleState>,
}

impl RenderPipelineCacheKey {
    pub fn new(shader_key: ShaderKey, layout_key: PipelineLayoutKey) -> Self {
        Self {
            shader_key,
            layout_key,
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            fragment_targets: Vec::new(),
            vertex_buffer_layouts: Vec::new(),
            vertex_constants: BTreeMap::new(),
            multisample: None,
        }
    }

    pub fn with_multisample(mut self, multisample: MultisampleState) -> Self {
        self.multisample = Some(multisample);
        self
    }

    pub fn with_push_vertex_buffer_layout(
        mut self,
        vertex_buffer_layout: VertexBufferLayout,
    ) -> Self {
        self.vertex_buffer_layouts.push(vertex_buffer_layout);
        self
    }

    pub fn with_push_fragment_target(mut self, target: ColorTargetState) -> Self {
        self.fragment_targets.push(target);
        self
    }

    pub fn with_push_fragment_targets(
        mut self,
        targets: impl IntoIterator<Item = ColorTargetState>,
    ) -> Self {
        for target in targets.into_iter() {
            self.fragment_targets.push(target);
        }
        self
    }

    pub fn with_primitive(mut self, primitive: PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }

    pub fn with_depth_stencil(mut self, depth_stencil: DepthStencilState) -> Self {
        self.depth_stencil = Some(depth_stencil);
        self
    }

    #[allow(dead_code)]
    pub fn with_vertex_constant(
        mut self,
        key: ConstantOverrideKey,
        value: ConstantOverrideValue,
    ) -> Self {
        self.vertex_constants.insert(key, value);
        self
    }
}

new_key_type! {
    pub struct RenderPipelineKey;
}

type Result<T> = std::result::Result<T, AwsmRenderPipelineError>;

#[derive(Error, Debug)]
pub enum AwsmRenderPipelineError {
    #[error("[render pipeline] missing pipeline: {0:?}")]
    NotFound(RenderPipelineKey),

    #[error("[render pipeline] missing shader: {0:?}")]
    MissingShader(ShaderKey),

    #[error("[render pipeline] bind group: {0:?}")]
    BindGroup(#[from] AwsmBindGroupError),

    #[error("[render pipeline]: {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[render pipeline] {0:?}")]
    Layout(#[from] AwsmPipelineLayoutError),
}
