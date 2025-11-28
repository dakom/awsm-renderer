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

pub static VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY: LazyLock<VertexBufferLayout> =
    LazyLock::new(|| {
        VertexBufferLayout {
            // this is the stride across all of the attributes
            // position (12) + triangle_index (4) + barycentric (8) + normal (12) + tangent (16) = 52 bytes
            array_stride: MeshBufferVertexInfo::BYTE_SIZE as u64,
            step_mode: None,
            attributes: vec![
                // Position (vec3<f32>) at offset 0
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                // Triangle ID (u32) at offset 12
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: 12,
                    shader_location: 1,
                },
                // Barycentric coordinates (vec2<f32>) at offset 16
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 16,
                    shader_location: 2,
                },
                // Normal (vec3<f32>) at offset 24
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 24,
                    shader_location: 3,
                },
                // Tangent (vec4<f32>) at offset 36
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 36,
                    shader_location: 4,
                },
            ],
        }
    });

pub static VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY_INSTANCING: LazyLock<VertexBufferLayout> =
    LazyLock::new(|| {
        let mut vertex_buffer_layout_instancing = VertexBufferLayout {
            // this is the stride across all of the attributes
            array_stride: MeshBufferVertexInfo::BYTE_SIZE_INSTANCE as u64,
            step_mode: Some(VertexStepMode::Instance),
            attributes: Vec::new(),
        };

        let start_location = VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY
            .attributes
            .len() as u32;

        for i in 0..4 {
            vertex_buffer_layout_instancing
                .attributes
                .push(VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: i * 16,
                    shader_location: start_location + i as u32,
                });
        }

        vertex_buffer_layout_instancing
    });

pub async fn geometry_and_transparency_render_pipeline_key(
    gpu: &AwsmRendererWebGpu,
    shaders: &mut Shaders,
    pipelines: &mut Pipelines,
    pipeline_layouts: &PipelineLayouts,
    depth_texture_format: TextureFormat,
    pipeline_layout_key: PipelineLayoutKey,
    shader_key: ShaderKey,
    color_targets: &[ColorTargetState],
    depth_write_enabled: bool,
    msaa_sample_count: Option<u32>,
    instancing: bool,
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
        .with_push_vertex_buffer_layout(VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY.clone())
        .with_depth_stencil(depth_stencil.clone());

    if instancing {
        pipeline_cache_key = pipeline_cache_key.with_push_vertex_buffer_layout(
            VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY_INSTANCING.clone(),
        );
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
