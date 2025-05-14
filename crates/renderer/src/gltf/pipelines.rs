use std::collections::BTreeMap;

use awsm_renderer_core::pipeline::constants::{ConstantOverrideKey, ConstantOverrideValue};
use awsm_renderer_core::pipeline::fragment::{ColorTargetState, FragmentState};
use awsm_renderer_core::pipeline::layout::{PipelineLayoutDescriptor, PipelineLayoutKind};
use awsm_renderer_core::pipeline::primitive::PrimitiveState;
use awsm_renderer_core::pipeline::vertex::{VertexBufferLayout, VertexState};
use awsm_renderer_core::pipeline::RenderPipelineDescriptor;

use crate::bind_groups::material_textures::MaterialBindGroupLayoutKey;
use crate::gltf::error::Result;

use crate::shaders::ShaderCacheKey;
use crate::AwsmRenderer;

use super::error::AwsmGltfError;

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub(crate) struct GltfRenderPipelineKey {
    pub shader_key: ShaderCacheKey,
    pub layout_key: GltfPipelineLayoutKey,
    pub primitive: PrimitiveState,
    pub fragment_targets: Vec<ColorTargetState>,
    pub vertex_buffer_layouts: Vec<VertexBufferLayout>,
    pub vertex_constants: BTreeMap<ConstantOverrideKey, ConstantOverrideValue>,
}

// merely a key to hash ad-hoc pipeline generation
#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GltfPipelineLayoutKey {
    pub has_morph_key: bool,
    pub has_skin_key: bool,
    pub material_layout_key: MaterialBindGroupLayoutKey,
}

impl GltfPipelineLayoutKey {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn into_descriptor(self, renderer: &AwsmRenderer) -> Result<PipelineLayoutDescriptor> {
        let mut bind_group_layouts = vec![
            renderer
                .bind_groups
                .uniform_storages
                .gpu_universal_bind_group_layout()
                .clone(),
            renderer
                .bind_groups
                .uniform_storages
                .gpu_mesh_all_bind_group_layout()
                .clone(),
            renderer
                .bind_groups
                .material_textures
                .gpu_bind_group_layout(self.material_layout_key)
                .map_err(AwsmGltfError::MaterialBindGroupLayout)?
                .clone(),
        ];

        if self.has_morph_key || self.has_skin_key {
            bind_group_layouts.push(
                renderer
                    .bind_groups
                    .uniform_storages
                    .gpu_mesh_shape_bind_group_layout()
                    .clone(),
            );
        }

        Ok(PipelineLayoutDescriptor::new(
            Some("Mesh (from gltf primitive)"),
            bind_group_layouts,
        ))
    }
}

impl GltfRenderPipelineKey {
    pub fn new(shader_key: ShaderCacheKey, layout_key: GltfPipelineLayoutKey) -> Self {
        Self {
            shader_key,
            layout_key,
            primitive: PrimitiveState::default(),
            fragment_targets: Vec::new(),
            vertex_buffer_layouts: Vec::new(),
            vertex_constants: BTreeMap::new(),
        }
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

    pub fn with_primitive(mut self, primitive: PrimitiveState) -> Self {
        self.primitive = primitive;
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

    pub fn into_descriptor(
        self,
        renderer: &mut AwsmRenderer,
        shader_module: &web_sys::GpuShaderModule,
    ) -> Result<web_sys::GpuRenderPipelineDescriptor> {
        let mut vertex = VertexState::new(shader_module, None);
        vertex.buffer_layouts = self.vertex_buffer_layouts;
        vertex.constants = self.vertex_constants;

        let fragment = FragmentState::new(shader_module, None, self.fragment_targets.clone());

        let layout = match renderer.gltf.pipeline_layouts.get(&self.layout_key) {
            None => {
                let layout = renderer.gpu.create_pipeline_layout(
                    &self.layout_key.clone().into_descriptor(renderer)?.into(),
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
                .with_primitive(self.primitive)
                .with_layout(layout)
                .with_fragment(fragment)
                .into(),
        )
    }
}
