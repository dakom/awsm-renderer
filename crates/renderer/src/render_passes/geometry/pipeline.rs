use awsm_renderer_core::compare::CompareFunction;
use awsm_renderer_core::pipeline::depth_stencil::DepthStencilState;
use awsm_renderer_core::pipeline::fragment::ColorTargetState;
use awsm_renderer_core::pipeline::primitive::{CullMode, FrontFace, PrimitiveState, PrimitiveTopology};
use awsm_renderer_core::pipeline::vertex::{VertexAttribute, VertexBufferLayout, VertexFormat};

use crate::error::Result;
use crate::pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey};
use crate::pipelines::render_pipeline::{RenderPipelineCacheKey, RenderPipelineKey};
use crate::render_passes::geometry::shader::cache_key::ShaderCacheKeyGeometry;
use crate::render_passes::{geometry::bind_group::GeometryBindGroups, RenderPassInitContext};

pub struct GeometryPipelines {
    pub pipeline_layout_key: PipelineLayoutKey,
    pub render_pipeline_key_cull_back: RenderPipelineKey,
    pub render_pipeline_key_cull_none: RenderPipelineKey,
}

impl GeometryPipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &GeometryBindGroups,
    ) -> Result<Self> {
        let pipeline_layout_cache_key = PipelineLayoutCacheKey::new(vec![
            bind_groups
                .camera_lights
                .bind_group_layout_key,
            bind_groups
                .transform_materials
                .bind_group_layout_key,
            bind_groups
                .vertex_animation
                .bind_group_layout_key,
        ]);

        let pipeline_layout_key = ctx.pipeline_layouts.get_key(
            &ctx.gpu,
            &ctx.bind_group_layouts,
            pipeline_layout_cache_key,
        )?;



        let shader_key = ctx.shaders.get_key(&ctx.gpu, ShaderCacheKeyGeometry {}).await?;

        let primitive_state_cull_back = PrimitiveState::new()
            .with_topology(PrimitiveTopology::TriangleList)
            .with_front_face(FrontFace::Ccw)
            .with_cull_mode(CullMode::Back);

        let primitive_state_cull_none = PrimitiveState::new()
            .with_topology(PrimitiveTopology::TriangleList)
            .with_front_face(FrontFace::Ccw)
            .with_cull_mode(CullMode::None);

        let color_targets = [
            // TODO - will revisit this to add more targets if necessary
            ColorTargetState::new(ctx.render_texture_formats.material_offset),
            ColorTargetState::new(ctx.render_texture_formats.world_normal),
            ColorTargetState::new(ctx.render_texture_formats.screen_pos),
            ColorTargetState::new(ctx.render_texture_formats.motion_vector),
        ];

        let vertex_buffer_layout = VertexBufferLayout {
            // this is the stride across all of the attributes
            array_stride: 24, // 24 bytes per vertex (12 pos + 4 triangle_id + 8 bary),
            step_mode: None,
            attributes: vec![
                // Position (vec3<f32>)
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                // Triangle ID (u32)
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: 12,
                    shader_location: 1,
                },
                // Barycentric coordinates (vec2<f32>)
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 16,
                    shader_location: 2,
                },
            ],
        };

        let mut pipeline_cache_key_cull_back = RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
            .with_primitive(primitive_state_cull_back)
            .with_push_vertex_buffer_layout(vertex_buffer_layout.clone())
            .with_depth_stencil(DepthStencilState::new(ctx.render_texture_formats.depth)
                    .with_depth_write_enabled(true)
                    .with_depth_compare(CompareFunction::LessEqual)
            )
            .with_push_fragment_targets(color_targets.clone());

        let mut pipeline_cache_key_cull_none = RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
            .with_primitive(primitive_state_cull_none)
            .with_push_vertex_buffer_layout(vertex_buffer_layout)
            .with_depth_stencil(DepthStencilState::new(ctx.render_texture_formats.depth)
                    .with_depth_write_enabled(true)
                    .with_depth_compare(CompareFunction::LessEqual)
            )
            .with_push_fragment_targets(color_targets);

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


        Ok(Self {
            pipeline_layout_key,
            render_pipeline_key_cull_back,
            render_pipeline_key_cull_none,
        })
    }

    pub fn get_render_pipeline_key(&self, double_sided: bool) -> RenderPipelineKey {
        match double_sided {
            true => self.render_pipeline_key_cull_none,
            false => self.render_pipeline_key_cull_back,
        }
    }
}
