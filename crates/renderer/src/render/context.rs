use awsm_renderer_core::command::{render_pass::RenderPassEncoder, CommandEncoder};

use crate::{bind_groups::BindGroups, instances::Instances, materials::Materials, mesh::Meshes, pipeline::{Pipelines, RenderPipelineKey}, skin::Skins, transform::Transforms};


pub struct RenderContext<'a> {
    pub command_encoder: CommandEncoder,
    pub render_pass: RenderPassEncoder,
    pub transforms: &'a Transforms,
    pub meshes: &'a Meshes,
    pub pipelines: &'a Pipelines,
    pub materials: &'a Materials,
    pub skins: &'a Skins,
    pub instances: &'a Instances,
    pub bind_groups: &'a BindGroups,
    pub last_render_pipeline_key: Option<RenderPipelineKey>,
}