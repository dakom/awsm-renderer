use awsm_renderer_core::compare::CompareFunction;
use awsm_renderer_core::pipeline::depth_stencil::DepthStencilState;
use awsm_renderer_core::pipeline::fragment::ColorTargetState;
use awsm_renderer_core::pipeline::primitive::{
    CullMode, FrontFace, PrimitiveState, PrimitiveTopology,
};
use awsm_renderer_core::pipeline::vertex::{
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::error::Result;
use crate::mesh::MeshBufferVertexInfo;
use crate::pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey};
use crate::pipelines::render_pipeline::{RenderPipelineCacheKey, RenderPipelineKey};
use crate::render_passes::geometry::shader::cache_key::ShaderCacheKeyGeometry;
use crate::render_passes::{geometry::bind_group::GeometryBindGroups, RenderPassInitContext};

pub struct GeometryPipelines {
    pub pipeline_layout_key: PipelineLayoutKey,
    pub render_pipeline_key_cull_back: RenderPipelineKey,
    pub render_pipeline_key_cull_back_instancing: RenderPipelineKey,
    pub render_pipeline_key_cull_none: RenderPipelineKey,
    pub render_pipeline_key_cull_none_instancing: RenderPipelineKey,
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

        let shader_key = ctx
            .shaders
            .get_key(
                &ctx.gpu,
                ShaderCacheKeyGeometry {
                    instancing_transforms: false,
                },
            )
            .await?;

        let shader_key_instancing = ctx
            .shaders
            .get_key(
                &ctx.gpu,
                ShaderCacheKeyGeometry {
                    instancing_transforms: true,
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
            ColorTargetState::new(ctx.render_texture_formats.visiblity_data),
            ColorTargetState::new(ctx.render_texture_formats.taa_clip_position),
            ColorTargetState::new(ctx.render_texture_formats.geometry_normal),
        ];

        // Only add separate tangent target in full-fidelity mode
        if ctx.render_texture_formats.use_separate_normal_tangent {
            color_targets.push(ColorTargetState::new(
                ctx.render_texture_formats.geometry_tangent,
            ));
        }

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

        let depth_stencil = DepthStencilState::new(ctx.render_texture_formats.depth)
            .with_depth_write_enabled(true)
            .with_depth_compare(CompareFunction::LessEqual);

        let mut pipeline_cache_key_cull_back =
            RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
                .with_primitive(primitive_state_cull_back.clone())
                .with_push_vertex_buffer_layout(vertex_buffer_layout.clone())
                .with_depth_stencil(depth_stencil.clone());

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

        for target in &color_targets {
            pipeline_cache_key_cull_back_instancing = pipeline_cache_key_cull_back_instancing
                .with_push_fragment_targets(vec![target.clone()]);
        }

        let mut pipeline_cache_key_cull_none =
            RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
                .with_primitive(primitive_state_cull_none.clone())
                .with_push_vertex_buffer_layout(vertex_buffer_layout.clone())
                .with_depth_stencil(depth_stencil.clone());

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

        for target in &color_targets {
            pipeline_cache_key_cull_none_instancing = pipeline_cache_key_cull_none_instancing
                .with_push_fragment_targets(vec![target.clone()]);
        }

        let render_pipeline_key_cull_back = ctx
            .pipelines
            .render
            .get_key(
                &ctx.gpu,
                &ctx.shaders,
                &ctx.pipeline_layouts,
                pipeline_cache_key_cull_back,
            )
            .await?;

        let render_pipeline_key_cull_back_instancing = ctx
            .pipelines
            .render
            .get_key(
                &ctx.gpu,
                &ctx.shaders,
                &ctx.pipeline_layouts,
                pipeline_cache_key_cull_back_instancing,
            )
            .await?;

        let render_pipeline_key_cull_none = ctx
            .pipelines
            .render
            .get_key(
                &ctx.gpu,
                &ctx.shaders,
                &ctx.pipeline_layouts,
                pipeline_cache_key_cull_none,
            )
            .await?;

        let render_pipeline_key_cull_none_instancing = ctx
            .pipelines
            .render
            .get_key(
                &ctx.gpu,
                &ctx.shaders,
                &ctx.pipeline_layouts,
                pipeline_cache_key_cull_none_instancing,
            )
            .await?;

        Ok(Self {
            pipeline_layout_key,
            render_pipeline_key_cull_back,
            render_pipeline_key_cull_back_instancing,
            render_pipeline_key_cull_none,
            render_pipeline_key_cull_none_instancing,
        })
    }

    pub fn get_render_pipeline_key(
        &self,
        double_sided: bool,
        transform_instancing: bool,
    ) -> RenderPipelineKey {
        match (double_sided, transform_instancing) {
            (true, false) => self.render_pipeline_key_cull_none,
            (false, false) => self.render_pipeline_key_cull_back,
            (true, true) => self.render_pipeline_key_cull_none_instancing,
            (false, true) => self.render_pipeline_key_cull_back_instancing,
        }
    }
}
