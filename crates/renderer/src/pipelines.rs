pub mod compute_pipeline;
pub mod render_pipeline;

use crate::pipelines::{compute_pipeline::ComputePipelines, render_pipeline::RenderPipelines};

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
