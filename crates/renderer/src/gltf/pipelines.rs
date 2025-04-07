use awsm_renderer_core::pipeline::fragment::{ColorTargetState, FragmentState};
use awsm_renderer_core::pipeline::layout::{PipelineLayoutDescriptor, PipelineLayoutKind};
use awsm_renderer_core::pipeline::vertex::{VertexBufferLayout, VertexState};
use awsm_renderer_core::pipeline::RenderPipelineDescriptor;

use crate::gltf::error::Result;

use crate::AwsmRenderer;

use super::shaders::ShaderKey;

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct PipelineKey {
    pub shader_key: ShaderKey,
    pub fragment_targets: Vec<ColorTargetState>,
    pub vertex_buffer_layouts: Vec<VertexBufferLayout>,
}

impl PipelineKey {
    pub fn new(
        renderer: &AwsmRenderer,
        shader_key: ShaderKey,
        vertex_buffer_layouts: Vec<VertexBufferLayout>,
    ) -> Self {
        Self {
            shader_key,
            fragment_targets: vec![ColorTargetState::new(renderer.gpu.current_context_format())],
            vertex_buffer_layouts,
        }
    }

    pub fn into_descriptor(
        self,
        renderer: &AwsmRenderer,
        shader_module: &web_sys::GpuShaderModule,
    ) -> Result<web_sys::GpuRenderPipelineDescriptor> {
        let fragment = FragmentState::new(shader_module, None, self.fragment_targets.clone());

        // TODO - re-use pipeline layouts (not just values, but, from a key lookup so we re-use the same objects)

        let layout = renderer.gpu.create_pipeline_layout(
            &PipelineLayoutDescriptor::new(
                Some("Mesh"),
                vec![renderer.camera.bind_group_layout.clone()],
            )
            .into(),
        );

        let layout = PipelineLayoutKind::Custom(layout);

        let mut vertex = VertexState::new(shader_module, None);
        vertex.buffers = self.vertex_buffer_layouts;

        Ok(RenderPipelineDescriptor::new(vertex, None)
            .with_layout(layout)
            .with_fragment(fragment)
            .into())
    }
}
