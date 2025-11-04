use awsm_renderer_core::compare::CompareFunction;
use awsm_renderer_core::pipeline::depth_stencil::DepthStencilState;
use awsm_renderer_core::pipeline::fragment::ColorTargetState;
use awsm_renderer_core::pipeline::multisample::MultisampleState;
use awsm_renderer_core::pipeline::primitive::{
    CullMode, FrontFace, PrimitiveState, PrimitiveTopology,
};
use awsm_renderer_core::pipeline::vertex::{
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, SlotMap};

use crate::anti_alias::AntiAliasing;
use crate::error::Result;
use crate::mesh::{MeshBufferInfos, MeshBufferVertexInfo};
use crate::pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey, PipelineLayouts};
use crate::pipelines::render_pipeline::{RenderPipelineCacheKey, RenderPipelineKey};
use crate::pipelines::Pipelines;
use crate::render_passes::geometry::shader::cache_key::ShaderCacheKeyGeometry;
use crate::render_passes::material::opaque::bind_group::MaterialOpaqueBindGroups;
use crate::render_passes::{geometry::bind_group::GeometryBindGroups, RenderPassInitContext};
use crate::render_textures::RenderTextureFormats;
use crate::shaders::Shaders;

pub struct GeometryPipelines {
    pub pipeline_layout_key: PipelineLayoutKey,
    no_anti_alias_pipeline_keys: GeometryPipelineKeys,
    msaa_4_pipeline_keys: GeometryPipelineKeys,
}

struct GeometryPipelineKeys {
    pub cull_back: RenderPipelineKey,
    pub cull_back_instancing: RenderPipelineKey,
    pub cull_none: RenderPipelineKey,
    pub cull_none_instancing: RenderPipelineKey,
}

impl GeometryPipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &GeometryBindGroups,
    ) -> Result<Self> {
        let pipeline_layout_cache_key = PipelineLayoutCacheKey::new(vec![
            bind_groups.camera.bind_group_layout_key,
            bind_groups.transform_materials.bind_group_layout_key,
            bind_groups.meta.bind_group_layout_key,
            bind_groups.animation.bind_group_layout_key,
        ]);

        let pipeline_layout_key = ctx.pipeline_layouts.get_key(
            &ctx.gpu,
            &ctx.bind_group_layouts,
            pipeline_layout_cache_key,
        )?;

        let no_anti_alias_pipeline_keys = init_pipeline_keys(
            pipeline_layout_key,
            None,
            &ctx.gpu,
            &mut ctx.shaders,
            &mut ctx.pipelines,
            &ctx.pipeline_layouts,
            &ctx.render_texture_formats,
        )
        .await?;

        let msaa_4_pipeline_keys = init_pipeline_keys(
            pipeline_layout_key,
            Some(4),
            &ctx.gpu,
            &mut ctx.shaders,
            &mut ctx.pipelines,
            &ctx.pipeline_layouts,
            &ctx.render_texture_formats,
        )
        .await?;

        Ok(Self {
            pipeline_layout_key,
            no_anti_alias_pipeline_keys,
            msaa_4_pipeline_keys,
        })
    }

    pub fn get_render_pipeline_key(
        &self,
        double_sided: bool,
        transform_instancing: bool,
        anti_aliasing: &AntiAliasing,
    ) -> RenderPipelineKey {
        let keys = match anti_aliasing.msaa_sample_count {
            Some(4) => &self.msaa_4_pipeline_keys,
            None => &self.no_anti_alias_pipeline_keys,
            _ => panic!("Unsupported MSAA sample count"),
        };

        match (double_sided, transform_instancing) {
            (true, false) => keys.cull_none,
            (false, false) => keys.cull_back,
            (true, true) => keys.cull_none_instancing,
            (false, true) => keys.cull_back_instancing,
        }
    }
}

async fn init_pipeline_keys(
    pipeline_layout_key: PipelineLayoutKey,
    msaa_sample_count: Option<u32>,
    gpu: &AwsmRendererWebGpu,
    shaders: &mut Shaders,
    pipelines: &mut Pipelines,
    pipeline_layouts: &PipelineLayouts,
    render_texture_formats: &RenderTextureFormats,
) -> Result<GeometryPipelineKeys> {
    let shader_key = shaders
        .get_key(
            &gpu,
            ShaderCacheKeyGeometry {
                instancing_transforms: false,
                msaa_samples: msaa_sample_count.unwrap_or_default(),
            },
        )
        .await?;

    let shader_key_instancing = shaders
        .get_key(
            &gpu,
            ShaderCacheKeyGeometry {
                instancing_transforms: true,
                msaa_samples: msaa_sample_count.unwrap_or_default(),
            },
        )
        .await?;

    let primitive_state_cull_back = PrimitiveState::new()
        .with_topology(PrimitiveTopology::TriangleList)
        .with_front_face(FrontFace::Ccw)
        .with_cull_mode(CullMode::Back);

    let primitive_state_cull_none = PrimitiveState::new()
        .with_topology(PrimitiveTopology::TriangleList)
        .with_front_face(FrontFace::Ccw)
        .with_cull_mode(CullMode::None);

    let mut color_targets = vec![
        ColorTargetState::new(render_texture_formats.visiblity_data),
        ColorTargetState::new(render_texture_formats.barycentric),
        ColorTargetState::new(render_texture_formats.geometry_normal),
        ColorTargetState::new(render_texture_formats.geometry_tangent),
    ];

    let vertex_buffer_layout = VertexBufferLayout {
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
    };

    let mut vertex_buffer_layout_instancing = VertexBufferLayout {
        // this is the stride across all of the attributes
        array_stride: MeshBufferVertexInfo::BYTE_SIZE_INSTANCE as u64,
        step_mode: Some(VertexStepMode::Instance),
        attributes: Vec::new(),
    };

    for i in 0..4 {
        vertex_buffer_layout_instancing
            .attributes
            .push(VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: i * 16,
                shader_location: 5 + i as u32, // Locations 5-8 (after normal at 3 and tangent at 4)
            });
    }

    let depth_stencil = DepthStencilState::new(render_texture_formats.depth)
        .with_depth_write_enabled(true)
        .with_depth_compare(CompareFunction::LessEqual);

    let mut pipeline_cache_key_cull_back =
        RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
            .with_primitive(primitive_state_cull_back.clone())
            .with_push_vertex_buffer_layout(vertex_buffer_layout.clone())
            .with_depth_stencil(depth_stencil.clone());

    if let Some(sample_count) = msaa_sample_count {
        pipeline_cache_key_cull_back = pipeline_cache_key_cull_back
            .with_multisample(MultisampleState::new().with_count(sample_count));
    }

    for target in &color_targets {
        pipeline_cache_key_cull_back =
            pipeline_cache_key_cull_back.with_push_fragment_targets(vec![target.clone()]);
    }

    let mut pipeline_cache_key_cull_back_instancing =
        RenderPipelineCacheKey::new(shader_key_instancing, pipeline_layout_key)
            .with_primitive(primitive_state_cull_back)
            .with_push_vertex_buffer_layout(vertex_buffer_layout.clone())
            .with_push_vertex_buffer_layout(vertex_buffer_layout_instancing.clone())
            .with_depth_stencil(depth_stencil.clone());

    if let Some(sample_count) = msaa_sample_count {
        pipeline_cache_key_cull_back_instancing = pipeline_cache_key_cull_back_instancing
            .with_multisample(MultisampleState::new().with_count(sample_count));
    }

    for target in &color_targets {
        pipeline_cache_key_cull_back_instancing = pipeline_cache_key_cull_back_instancing
            .with_push_fragment_targets(vec![target.clone()]);
    }

    let mut pipeline_cache_key_cull_none =
        RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
            .with_primitive(primitive_state_cull_none.clone())
            .with_push_vertex_buffer_layout(vertex_buffer_layout.clone())
            .with_depth_stencil(depth_stencil.clone());

    if let Some(sample_count) = msaa_sample_count {
        pipeline_cache_key_cull_none = pipeline_cache_key_cull_none
            .with_multisample(MultisampleState::new().with_count(sample_count));
    }

    for target in &color_targets {
        pipeline_cache_key_cull_none =
            pipeline_cache_key_cull_none.with_push_fragment_targets(vec![target.clone()]);
    }

    let mut pipeline_cache_key_cull_none_instancing =
        RenderPipelineCacheKey::new(shader_key_instancing, pipeline_layout_key)
            .with_primitive(primitive_state_cull_none)
            .with_push_vertex_buffer_layout(vertex_buffer_layout)
            .with_push_vertex_buffer_layout(vertex_buffer_layout_instancing)
            .with_depth_stencil(depth_stencil);

    if let Some(sample_count) = msaa_sample_count {
        pipeline_cache_key_cull_none_instancing = pipeline_cache_key_cull_none_instancing
            .with_multisample(MultisampleState::new().with_count(sample_count));
    }

    for target in &color_targets {
        pipeline_cache_key_cull_none_instancing = pipeline_cache_key_cull_none_instancing
            .with_push_fragment_targets(vec![target.clone()]);
    }

    let render_pipeline_key_cull_back = pipelines
        .render
        .get_key(
            &gpu,
            &shaders,
            &pipeline_layouts,
            pipeline_cache_key_cull_back,
        )
        .await?;

    let render_pipeline_key_cull_back_instancing = pipelines
        .render
        .get_key(
            &gpu,
            &shaders,
            &pipeline_layouts,
            pipeline_cache_key_cull_back_instancing,
        )
        .await?;

    let render_pipeline_key_cull_none = pipelines
        .render
        .get_key(
            &gpu,
            &shaders,
            &pipeline_layouts,
            pipeline_cache_key_cull_none,
        )
        .await?;

    let render_pipeline_key_cull_none_instancing = pipelines
        .render
        .get_key(
            &gpu,
            &shaders,
            &pipeline_layouts,
            pipeline_cache_key_cull_none_instancing,
        )
        .await?;

    Ok(GeometryPipelineKeys {
        cull_back: render_pipeline_key_cull_back,
        cull_back_instancing: render_pipeline_key_cull_back_instancing,
        cull_none: render_pipeline_key_cull_none,
        cull_none_instancing: render_pipeline_key_cull_none_instancing,
    })
}
