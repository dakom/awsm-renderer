use std::sync::LazyLock;

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
use crate::render_passes::shared::geometry_and_transparency::vertex::geometry_and_transparency_render_pipeline_key;
use crate::render_passes::{geometry::bind_group::GeometryBindGroups, RenderPassInitContext};
use crate::render_textures::RenderTextureFormats;
use crate::shaders::Shaders;

pub struct GeometryPipelines {
    pub pipeline_layout_key: PipelineLayoutKey,
    no_anti_alias_no_cull_no_instancing_render_pipeline_key: RenderPipelineKey,
    no_anti_alias_no_cull_instancing_render_pipeline_key: RenderPipelineKey,
    no_anti_alias_back_cull_no_instancing_render_pipeline_key: RenderPipelineKey,
    no_anti_alias_back_cull_instancing_render_pipeline_key: RenderPipelineKey,
    msaa_4_anti_alias_no_cull_no_instancing_render_pipeline_key: RenderPipelineKey,
    msaa_4_anti_alias_no_cull_instancing_render_pipeline_key: RenderPipelineKey,
    msaa_4_anti_alias_back_cull_no_instancing_render_pipeline_key: RenderPipelineKey,
    msaa_4_anti_alias_back_cull_instancing_render_pipeline_key: RenderPipelineKey,
}

static VERTEX_BUFFER_LAYOUT: LazyLock<VertexBufferLayout> = LazyLock::new(|| {
    VertexBufferLayout {
        // this is the stride across all of the attributes
        // position (12) + triangle_index (4) + barycentric (8) + normal (12) + tangent (16) = 52 bytes
        array_stride: MeshBufferVertexInfo::VISIBILITY_GEOMETRY_BYTE_SIZE as u64,
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

static VERTEX_BUFFER_LAYOUT_INSTANCING: LazyLock<VertexBufferLayout> = LazyLock::new(|| {
    let mut vertex_buffer_layout_instancing = VertexBufferLayout {
        // this is the stride across all of the attributes
        array_stride: MeshBufferVertexInfo::INSTANCING_BYTE_SIZE as u64,
        step_mode: Some(VertexStepMode::Instance),
        attributes: Vec::new(),
    };

    let start_location = VERTEX_BUFFER_LAYOUT.attributes.len() as u32;

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

        let color_targets = &[
            ColorTargetState::new(ctx.render_texture_formats.visiblity_data),
            ColorTargetState::new(ctx.render_texture_formats.barycentric),
            ColorTargetState::new(ctx.render_texture_formats.normal_tangent),
            ColorTargetState::new(ctx.render_texture_formats.barycentric_derivatives),
        ];

        let vertex_buffer_layouts_no_instancing = vec![VERTEX_BUFFER_LAYOUT.clone()];
        let vertex_buffer_layouts_instancing = vec![
            VERTEX_BUFFER_LAYOUT.clone(),
            VERTEX_BUFFER_LAYOUT_INSTANCING.clone(),
        ];

        let shader_key_no_anti_alias_no_instancing = ctx
            .shaders
            .get_key(
                &ctx.gpu,
                ShaderCacheKeyGeometry {
                    instancing_transforms: false,
                    msaa_samples: None,
                },
            )
            .await?;
        let shader_key_no_anti_alias_instancing = ctx
            .shaders
            .get_key(
                &ctx.gpu,
                ShaderCacheKeyGeometry {
                    instancing_transforms: true,
                    msaa_samples: None,
                },
            )
            .await?;

        let shader_key_msaa_4_anti_alias_no_instancing = ctx
            .shaders
            .get_key(
                &ctx.gpu,
                ShaderCacheKeyGeometry {
                    instancing_transforms: false,
                    msaa_samples: Some(4),
                },
            )
            .await?;
        let shader_key_msaa_4_anti_alias_instancing = ctx
            .shaders
            .get_key(
                &ctx.gpu,
                ShaderCacheKeyGeometry {
                    instancing_transforms: true,
                    msaa_samples: Some(4),
                },
            )
            .await?;

        let no_anti_alias_no_cull_no_instancing_render_pipeline_key =
            geometry_and_transparency_render_pipeline_key(
                &ctx.gpu,
                &mut ctx.shaders,
                &mut ctx.pipelines,
                &ctx.pipeline_layouts,
                ctx.render_texture_formats.depth,
                pipeline_layout_key,
                shader_key_no_anti_alias_no_instancing,
                vertex_buffer_layouts_no_instancing.clone(),
                color_targets,
                true,
                None,
                CullMode::None,
                None,
            )
            .await?;

        let no_anti_alias_no_cull_instancing_render_pipeline_key =
            geometry_and_transparency_render_pipeline_key(
                &ctx.gpu,
                &mut ctx.shaders,
                &mut ctx.pipelines,
                &ctx.pipeline_layouts,
                ctx.render_texture_formats.depth,
                pipeline_layout_key,
                shader_key_no_anti_alias_instancing,
                vertex_buffer_layouts_instancing.clone(),
                color_targets,
                true,
                None,
                CullMode::None,
                None,
            )
            .await?;

        let no_anti_alias_back_cull_no_instancing_render_pipeline_key =
            geometry_and_transparency_render_pipeline_key(
                &ctx.gpu,
                &mut ctx.shaders,
                &mut ctx.pipelines,
                &ctx.pipeline_layouts,
                ctx.render_texture_formats.depth,
                pipeline_layout_key,
                shader_key_no_anti_alias_no_instancing,
                vertex_buffer_layouts_no_instancing.clone(),
                color_targets,
                true,
                None,
                CullMode::Back,
                None,
            )
            .await?;

        let no_anti_alias_back_cull_instancing_render_pipeline_key =
            geometry_and_transparency_render_pipeline_key(
                &ctx.gpu,
                &mut ctx.shaders,
                &mut ctx.pipelines,
                &ctx.pipeline_layouts,
                ctx.render_texture_formats.depth,
                pipeline_layout_key,
                shader_key_no_anti_alias_instancing,
                vertex_buffer_layouts_instancing.clone(),
                color_targets,
                true,
                None,
                CullMode::Back,
                None,
            )
            .await?;

        let msaa_4_anti_alias_no_cull_no_instancing_render_pipeline_key =
            geometry_and_transparency_render_pipeline_key(
                &ctx.gpu,
                &mut ctx.shaders,
                &mut ctx.pipelines,
                &ctx.pipeline_layouts,
                ctx.render_texture_formats.depth,
                pipeline_layout_key,
                shader_key_msaa_4_anti_alias_no_instancing,
                vertex_buffer_layouts_no_instancing.clone(),
                color_targets,
                true,
                Some(4),
                CullMode::None,
                None,
            )
            .await?;

        let msaa_4_anti_alias_no_cull_instancing_render_pipeline_key =
            geometry_and_transparency_render_pipeline_key(
                &ctx.gpu,
                &mut ctx.shaders,
                &mut ctx.pipelines,
                &ctx.pipeline_layouts,
                ctx.render_texture_formats.depth,
                pipeline_layout_key,
                shader_key_msaa_4_anti_alias_instancing,
                vertex_buffer_layouts_instancing.clone(),
                color_targets,
                true,
                Some(4),
                CullMode::None,
                None,
            )
            .await?;

        let msaa_4_anti_alias_back_cull_no_instancing_render_pipeline_key =
            geometry_and_transparency_render_pipeline_key(
                &ctx.gpu,
                &mut ctx.shaders,
                &mut ctx.pipelines,
                &ctx.pipeline_layouts,
                ctx.render_texture_formats.depth,
                pipeline_layout_key,
                shader_key_msaa_4_anti_alias_no_instancing,
                vertex_buffer_layouts_no_instancing.clone(),
                color_targets,
                true,
                Some(4),
                CullMode::Back,
                None,
            )
            .await?;

        let msaa_4_anti_alias_back_cull_instancing_render_pipeline_key =
            geometry_and_transparency_render_pipeline_key(
                &ctx.gpu,
                &mut ctx.shaders,
                &mut ctx.pipelines,
                &ctx.pipeline_layouts,
                ctx.render_texture_formats.depth,
                pipeline_layout_key,
                shader_key_msaa_4_anti_alias_instancing,
                vertex_buffer_layouts_instancing.clone(),
                color_targets,
                true,
                Some(4),
                CullMode::Back,
                None,
            )
            .await?;

        Ok(Self {
            pipeline_layout_key,
            no_anti_alias_no_cull_no_instancing_render_pipeline_key,
            no_anti_alias_no_cull_instancing_render_pipeline_key,
            no_anti_alias_back_cull_no_instancing_render_pipeline_key,
            no_anti_alias_back_cull_instancing_render_pipeline_key,
            msaa_4_anti_alias_no_cull_no_instancing_render_pipeline_key,
            msaa_4_anti_alias_no_cull_instancing_render_pipeline_key,
            msaa_4_anti_alias_back_cull_no_instancing_render_pipeline_key,
            msaa_4_anti_alias_back_cull_instancing_render_pipeline_key,
        })
    }

    pub fn get_render_pipeline_key(
        &self,
        double_sided: bool,
        transform_instancing: bool,
        anti_aliasing: &AntiAliasing,
    ) -> RenderPipelineKey {
        let has_anti_alias = match anti_aliasing.msaa_sample_count {
            Some(4) => true,
            None => false,
            _ => panic!("Unsupported MSAA sample count"),
        };

        match (has_anti_alias, double_sided, transform_instancing) {
            (false, false, false) => self.no_anti_alias_back_cull_no_instancing_render_pipeline_key,
            (false, false, true) => self.no_anti_alias_back_cull_instancing_render_pipeline_key,
            (false, true, false) => self.no_anti_alias_no_cull_no_instancing_render_pipeline_key,
            (false, true, true) => self.no_anti_alias_no_cull_instancing_render_pipeline_key,
            (true, false, false) => {
                self.msaa_4_anti_alias_back_cull_no_instancing_render_pipeline_key
            }
            (true, false, true) => self.msaa_4_anti_alias_back_cull_instancing_render_pipeline_key,
            (true, true, false) => self.msaa_4_anti_alias_no_cull_no_instancing_render_pipeline_key,
            (true, true, true) => self.msaa_4_anti_alias_no_cull_instancing_render_pipeline_key,
        }
    }
}
