use awsm_renderer_core::pipeline::{
    fragment::{ColorTargetState, FragmentState},
    vertex::{VertexBufferLayout, VertexState},
    RenderPipelineDescriptor,
};

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
    pub fn new(renderer: &AwsmRenderer, shader_key: ShaderKey) -> Self {
        Self {
            shader_key,
            fragment_targets: vec![ColorTargetState::new(renderer.gpu.current_context_format())],
            vertex_buffer_layouts: Vec::new()
        }
    }

    pub fn into_descriptor(
        self,
        shader_module: &web_sys::GpuShaderModule,
    ) -> web_sys::GpuRenderPipelineDescriptor {
        let mut vertex = VertexState::new(shader_module, None);
        vertex.buffers = self.vertex_buffer_layouts;

        let fragment = FragmentState::new(shader_module, None, self.fragment_targets.clone());

        RenderPipelineDescriptor::new(vertex, None)
            .with_fragment(fragment)
            .into()
    }
}
