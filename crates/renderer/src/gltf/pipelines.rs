use awsm_renderer_core::pipeline::fragment::{ColorTargetState, FragmentState};
use awsm_renderer_core::pipeline::layout::{PipelineLayoutDescriptor, PipelineLayoutKind};
use awsm_renderer_core::pipeline::vertex::{VertexBufferLayout, VertexState};
use awsm_renderer_core::pipeline::RenderPipelineDescriptor;

use crate::gltf::error::Result;

use crate::AwsmRenderer;

use super::shaders::ShaderKey;

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct RenderPipelineKey {
    pub shader_key: ShaderKey,
    pub layout_key: PipelineLayoutKey,
    pub fragment_targets: Vec<ColorTargetState>,
    pub vertex_buffer_layouts: Vec<VertexBufferLayout>,
}

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub struct PipelineLayoutKey {
    pub camera: bool,
}

impl PipelineLayoutKey {
    pub fn with_camera(mut self) -> Self {
        self.camera = true;
        self
    }

    pub fn into_descriptor(self, renderer: &AwsmRenderer) -> PipelineLayoutDescriptor {
        PipelineLayoutDescriptor::new(
            None,
            match self.camera {
                true => vec![renderer.camera.bind_group_layout.clone()],
                false => Vec::new(),
            },
        )
    }
}

impl RenderPipelineKey {
    pub fn new(
        renderer: &AwsmRenderer,
        shader_key: ShaderKey,
        layout_key: PipelineLayoutKey,
        vertex_buffer_layouts: Vec<VertexBufferLayout>,
    ) -> Self {
        Self {
            shader_key,
            layout_key,
            fragment_targets: vec![ColorTargetState::new(renderer.gpu.current_context_format())],
            vertex_buffer_layouts,
        }
    }

    pub fn into_descriptor(
        self,
        renderer: &mut AwsmRenderer,
        shader_module: &web_sys::GpuShaderModule,
    ) -> Result<web_sys::GpuRenderPipelineDescriptor> {
        let vertex =
            VertexState::new(shader_module, None).with_buffer_layouts(self.vertex_buffer_layouts);

        let fragment = FragmentState::new(shader_module, None, self.fragment_targets.clone());

        let layout = match renderer.gltf.pipeline_layouts.get(&self.layout_key) {
            None => {
                let layout = renderer.gpu.create_pipeline_layout(
                    &self.layout_key.clone().into_descriptor(renderer).into(),
                );

                renderer
                    .gltf
                    .pipeline_layouts
                    .insert(self.layout_key, layout.clone());

                layout
            }
            Some(layout) => layout.clone(),
        };

        let layout = PipelineLayoutKind::Custom(layout);

        Ok(RenderPipelineDescriptor::new(vertex, None)
            .with_layout(layout)
            .with_fragment(fragment)
            .into())
    }
}
