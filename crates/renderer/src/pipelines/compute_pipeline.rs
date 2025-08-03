use std::collections::{BTreeMap, HashMap};

use awsm_renderer_core::{error::AwsmCoreError, pipeline::{constants::{ConstantOverrideKey, ConstantOverrideValue}, depth_stencil::DepthStencilState, fragment::{ColorTargetState, FragmentState}, layout::PipelineLayoutKind, primitive::PrimitiveState, vertex::{VertexBufferLayout, VertexState}, ComputePipelineDescriptor, ProgrammableStage, RenderPipelineDescriptor}, renderer::AwsmRendererWebGpu};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{bind_groups::AwsmBindGroupError, pipeline_layouts::{AwsmPipelineLayoutError, PipelineLayoutKey, PipelineLayouts}, shaders::{ShaderKey, Shaders}};

pub struct ComputePipelines {
    lookup: SlotMap<ComputePipelineKey, web_sys::GpuComputePipeline>,
    cache: HashMap<ComputePipelineCacheKey, ComputePipelineKey>,
}

impl ComputePipelines {
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
        cache_key: ComputePipelineCacheKey,
    ) -> Result<ComputePipelineKey> {
        if let Some(key) = self.cache.get(&cache_key) {
            return Ok(*key);
        }

        let cache_key_clone = cache_key.clone();

        let shader_module = shaders
            .get(cache_key.shader_key)
            .ok_or(AwsmComputePipelineError::MissingShader(cache_key.shader_key))?;

        let layout = pipeline_layouts.get(cache_key.layout_key)?;

        let mut programmable_stage = ProgrammableStage::new(shader_module, None);
        programmable_stage.constant_overrides = cache_key.constant_overrides;

        let mut descriptor = ComputePipelineDescriptor::new(programmable_stage, PipelineLayoutKind::Custom(layout), None);

        let pipeline = gpu
            .create_compute_pipeline(&descriptor.into())
            .await?;

        let key = self.lookup.insert(pipeline);
        self.cache.insert(cache_key_clone, key);
        Ok(key)
    }

    pub fn get(
        &self,
        key: ComputePipelineKey,
    ) -> Result<&web_sys::GpuComputePipeline> {
        self.lookup.get(key).ok_or(AwsmComputePipelineError::NotFound(key))
    }
}

impl Default for ComputePipelines {
    fn default() -> Self {
        Self::new()
    }
}

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ComputePipelineCacheKey {
    pub shader_key: ShaderKey,
    pub layout_key: PipelineLayoutKey,
    pub constant_overrides: BTreeMap<ConstantOverrideKey, ConstantOverrideValue>,
}

impl ComputePipelineCacheKey {
    pub fn new(shader_key: ShaderKey, layout_key: PipelineLayoutKey) -> Self {
        Self {
            shader_key,
            layout_key,
            constant_overrides: BTreeMap::new(),
        }
    }

    pub fn with_push_constant_override(
        mut self,
        key: ConstantOverrideKey,
        value: ConstantOverrideValue,
    ) -> Self {
        self.constant_overrides.insert(key, value);
        self
    }
}

new_key_type! {
    pub struct ComputePipelineKey;
}

type Result<T> = std::result::Result<T, AwsmComputePipelineError>;

#[derive(Error, Debug)]
pub enum AwsmComputePipelineError {
    #[error("[compute pipeline] missing pipeline: {0:?}")]
    NotFound(ComputePipelineKey),

    #[error("[compute pipeline] missing shader: {0:?}")]
    MissingShader(ShaderKey),

    #[error("[compute pipeline] bind group: {0:?}")]
    BindGroup(#[from] AwsmBindGroupError),

    #[error("[compute pipeline]: {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[compute pipeline] {0:?}")]
    Layout(#[from] AwsmPipelineLayoutError)
}