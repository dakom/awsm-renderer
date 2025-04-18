use std::collections::BTreeMap;

use awsm_renderer_core::pipeline::constants::{ConstantOverrideKey, ConstantOverrideValue};
use awsm_renderer_core::pipeline::fragment::{ColorTargetState, FragmentState};
use awsm_renderer_core::pipeline::layout::{PipelineLayoutDescriptor, PipelineLayoutKind};
use awsm_renderer_core::pipeline::vertex::{VertexBufferLayout, VertexState};
use awsm_renderer_core::pipeline::RenderPipelineDescriptor;

use crate::buffers::storage::StorageBufferKey;
use crate::gltf::error::Result;

use crate::mesh::{MeshBufferInfo, MorphKey};
use crate::shaders::ShaderKey;
use crate::AwsmRenderer;

use super::populate::GltfPopulateContext;

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderPipelineKey {
    pub shader_key: ShaderKey,
    pub layout_key: PipelineLayoutKey,
    pub fragment_targets: Vec<ColorTargetState>,
    pub vertex_buffer_layouts: Vec<VertexBufferLayout>,
    pub vertex_constants: BTreeMap<ConstantOverrideKey, ConstantOverrideValue>,
}

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PipelineLayoutKey {
    pub morph_buffer_storage_key: Option<StorageBufferKey>,
    pub morph_targets_len: Option<usize>, // TODO - override constant in shader
}

impl PipelineLayoutKey {
    #[allow(private_interfaces)]
    pub fn new(ctx: &GltfPopulateContext, buffer_info: &MeshBufferInfo) -> Self {
        let mut key = Self::default();

        if let Some(morph) = buffer_info.morph.as_ref() {
            key.morph_targets_len = Some(morph.targets_len);
            key.morph_buffer_storage_key = Some(ctx.morph_buffer_storage_key.unwrap());
        }

        key
    }
}

impl PipelineLayoutKey {
    pub fn into_descriptor(
        self,
        renderer: &AwsmRenderer,
        morph_key: Option<MorphKey>,
    ) -> Result<PipelineLayoutDescriptor> {
        let mut bind_group_layouts = vec![
            renderer.camera.bind_group_layout.clone(),
            renderer.transforms.bind_group_layout().clone(),
        ];

        if let Some(morph_key) = morph_key {
            bind_group_layouts.push(renderer.meshes.morphs.weights_bind_group_layout().clone());
            bind_group_layouts.push(
                renderer
                    .meshes
                    .morphs
                    .values_bind_group_layout(morph_key)?
                    .clone(),
            );
        }
        Ok(PipelineLayoutDescriptor::new(
            Some("Mesh (from gltf primitive)"),
            bind_group_layouts,
        ))
    }
}

impl RenderPipelineKey {
    pub fn new(shader_key: ShaderKey, layout_key: PipelineLayoutKey) -> Self {
        Self {
            shader_key,
            layout_key,
            fragment_targets: Vec::new(),
            vertex_buffer_layouts: Vec::new(),
            vertex_constants: BTreeMap::new(),
        }
    }

    pub fn with_vertex_buffer_layout(mut self, vertex_buffer_layout: VertexBufferLayout) -> Self {
        self.vertex_buffer_layouts.push(vertex_buffer_layout);
        self
    }

    pub fn with_fragment_target(mut self, target: ColorTargetState) -> Self {
        self.fragment_targets.push(target);
        self
    }

    pub fn with_vertex_constant(
        mut self,
        key: ConstantOverrideKey,
        value: ConstantOverrideValue,
    ) -> Self {
        self.vertex_constants.insert(key, value);
        self
    }

    pub fn into_descriptor(
        self,
        renderer: &mut AwsmRenderer,
        shader_module: &web_sys::GpuShaderModule,
        morph_key: Option<MorphKey>,
    ) -> Result<web_sys::GpuRenderPipelineDescriptor> {
        let mut vertex = VertexState::new(shader_module, None);
        vertex.buffer_layouts = self.vertex_buffer_layouts;
        vertex.constants = self.vertex_constants;

        let fragment = FragmentState::new(shader_module, None, self.fragment_targets.clone());

        let layout = match renderer.gltf.pipeline_layouts.get(&self.layout_key) {
            None => {
                let layout = renderer.gpu.create_pipeline_layout(
                    &self
                        .layout_key
                        .clone()
                        .into_descriptor(renderer, morph_key)?
                        .into(),
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

        Ok(
            RenderPipelineDescriptor::new(vertex, Some("Mesh (from gltf primitive)"))
                .with_layout(layout)
                .with_fragment(fragment)
                .into(),
        )
    }
}
