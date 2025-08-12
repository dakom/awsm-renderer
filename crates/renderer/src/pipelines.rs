pub mod compute_pipeline;
pub mod render_pipeline;
use std::collections::{BTreeMap, HashMap};

use awsm_renderer_core::{
    error::AwsmCoreError,
    pipeline::{
        constants::{ConstantOverrideKey, ConstantOverrideValue},
        depth_stencil::DepthStencilState,
        fragment::ColorTargetState,
        primitive::PrimitiveState,
        vertex::VertexBufferLayout,
    },
};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::AwsmBindGroupError,
    pipeline_layouts::PipelineLayoutKey,
    pipelines::{compute_pipeline::ComputePipelines, render_pipeline::RenderPipelines},
    shaders::ShaderKey,
};

pub struct Pipelines {
    pub render: RenderPipelines,
    pub compute: ComputePipelines,
}

impl Pipelines {
    pub fn new() -> Self {
        Self {
            render: RenderPipelines::new(),
            compute: ComputePipelines::new(),
        }
    }
}

impl Default for Pipelines {
    fn default() -> Self {
        Self::new()
    }
}
