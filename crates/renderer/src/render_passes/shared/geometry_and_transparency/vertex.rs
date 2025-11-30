use crate::{
    error::Result,
    pipeline_layouts::{PipelineLayoutKey, PipelineLayouts},
    pipelines::{render_pipeline::RenderPipelineKey, Pipelines},
    render_textures::RenderTextureFormats,
    shaders::Shaders,
};
use std::sync::LazyLock;

use awsm_renderer_core::{
    compare::CompareFunction,
    pipeline::{
        depth_stencil::DepthStencilState,
        fragment::ColorTargetState,
        multisample::MultisampleState,
        primitive::{CullMode, FrontFace, PrimitiveState, PrimitiveTopology},
        vertex::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode},
    },
    renderer::AwsmRendererWebGpu,
    texture::TextureFormat,
};

use crate::{
    mesh::MeshBufferVertexInfo, pipelines::render_pipeline::RenderPipelineCacheKey,
    shaders::ShaderKey,
};

pub async fn geometry_and_transparency_render_pipeline_key(
    gpu: &AwsmRendererWebGpu,
    shaders: &mut Shaders,
    pipelines: &mut Pipelines,
    pipeline_layouts: &PipelineLayouts,
    depth_texture_format: TextureFormat,
    pipeline_layout_key: PipelineLayoutKey,
    shader_key: ShaderKey,
    vertex_buffer_layouts: Vec<VertexBufferLayout>,
    color_targets: &[ColorTargetState],
    depth_write_enabled: bool,
    msaa_sample_count: Option<u32>,
    cull_mode: CullMode,
    transparency_buffer_layout: Option<VertexBufferLayout>,
) -> Result<RenderPipelineKey> {
    let primitive_state = PrimitiveState::new()
        .with_topology(PrimitiveTopology::TriangleList)
        .with_front_face(FrontFace::Ccw)
        .with_cull_mode(cull_mode);

    let depth_stencil = DepthStencilState::new(depth_texture_format)
        .with_depth_write_enabled(depth_write_enabled)
        .with_depth_compare(CompareFunction::LessEqual);

    let mut pipeline_cache_key = RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
        .with_primitive(primitive_state.clone())
        .with_depth_stencil(depth_stencil.clone());

    for layout in vertex_buffer_layouts {
        pipeline_cache_key = pipeline_cache_key.with_push_vertex_buffer_layout(layout);
    }

    if let Some(buffer_layout) = transparency_buffer_layout {
        pipeline_cache_key =
            pipeline_cache_key.with_push_vertex_buffer_layout(buffer_layout.clone());
    }

    if let Some(sample_count) = msaa_sample_count {
        pipeline_cache_key =
            pipeline_cache_key.with_multisample(MultisampleState::new().with_count(sample_count));
    }

    for target in color_targets {
        pipeline_cache_key = pipeline_cache_key.with_push_fragment_targets(vec![target.clone()]);
    }

    Ok(pipelines
        .render
        .get_key(&gpu, &shaders, &pipeline_layouts, pipeline_cache_key)
        .await?)
}
