use awsm_renderer_core::pipeline::fragment::{ColorTargetState, FragmentState};
use awsm_renderer_core::pipeline::layout::PipelineLayoutKind;
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
        _renderer: &AwsmRenderer,
        shader_module: &web_sys::GpuShaderModule,
    ) -> Result<web_sys::GpuRenderPipelineDescriptor> {
        let fragment = FragmentState::new(shader_module, None, self.fragment_targets.clone());

        let layout = if self.vertex_buffer_layouts.is_empty() {
            PipelineLayoutKind::Auto
        } else {
            PipelineLayoutKind::Auto
            // let mut pipeline_layout_descriptor = PipelineLayoutDescriptor::new(None);

            // for binding in 0..self.vertex_buffer_layouts.len() {
            //     let buffer_layout = BufferBindingLayout::default().with_binding_type(BufferBindingType::ReadOnlyStorage);
            //     let bind_group_layout_descriptor = BindGroupLayoutDescriptor{
            //         entries: vec![
            //             BindGroupLayoutEntry::new(binding as u32, BindGroupLayoutResource::Buffer(buffer_layout))
            //             .with_visibility_vertex()
            //         ],
            //         ..Default::default()
            //     };
            //     let bind_group_layout = renderer.gpu.create_bind_group_layout(&bind_group_layout_descriptor.into()).map_err(AwsmGltfError::BindGroupLayout)?;
            //     pipeline_layout_descriptor.bind_group_layouts.push(bind_group_layout);
            // }

            // let pipeline_layout = renderer.gpu.create_pipeline_layout(&pipeline_layout_descriptor.into());

            // PipelineLayoutKind::Custom(pipeline_layout)
        };

        let mut vertex = VertexState::new(shader_module, None);
        vertex.buffers = self.vertex_buffer_layouts;

        Ok(RenderPipelineDescriptor::new(vertex, None)
            .with_layout(layout)
            .with_fragment(fragment)
            .into())
    }
}
