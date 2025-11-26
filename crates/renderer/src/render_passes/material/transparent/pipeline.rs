use awsm_renderer_core::pipeline::fragment::ColorTargetState;
use awsm_renderer_core::pipeline::primitive::CullMode;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::SecondaryMap;

use crate::anti_alias::AntiAliasing;
use crate::error::Result;
use crate::materials::{MaterialKey, Materials};
use crate::mesh::{Mesh, MeshBufferInfoKey, MeshBufferInfos, MeshKey, Meshes};
use crate::pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey, PipelineLayouts};
use crate::pipelines::render_pipeline::RenderPipelineKey;
use crate::pipelines::Pipelines;
use crate::render_passes::material::cache_key::ShaderCacheKeyMaterial;
use crate::render_passes::material::transparent::shader::cache_key::ShaderCacheKeyMaterialTransparent;
use crate::render_passes::shared::geometry_and_transparency::vertex::geometry_and_transparency_render_pipeline_key;
use crate::render_passes::{
    material::transparent::bind_group::MaterialTransparentBindGroups, RenderPassInitContext,
};
use crate::render_textures::RenderTextureFormats;
use crate::shaders::Shaders;
use crate::textures::Textures;

pub struct MaterialTransparentPipelines {
    multisampled_pipeline_layout_key: PipelineLayoutKey,
    singlesampled_pipeline_layout_key: PipelineLayoutKey,
    render_pipeline_keys: SecondaryMap<MeshKey, RenderPipelineKey>,
}

impl MaterialTransparentPipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &MaterialTransparentBindGroups,
    ) -> Result<Self> {
        let multisampled_pipeline_layout_cache_key = PipelineLayoutCacheKey::new(vec![
            bind_groups.multisampled_main_bind_group_layout_key,
            bind_groups.lights_bind_group_layout_key,
            bind_groups.texture_pool_textures_bind_group_layout_key,
            bind_groups.texture_pool_samplers_bind_group_layout_key,
        ]);
        let multisampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            &ctx.gpu,
            &ctx.bind_group_layouts,
            multisampled_pipeline_layout_cache_key,
        )?;

        let singlesampled_pipeline_layout_cache_key = PipelineLayoutCacheKey::new(vec![
            bind_groups.singlesampled_main_bind_group_layout_key,
            bind_groups.lights_bind_group_layout_key,
            bind_groups.texture_pool_textures_bind_group_layout_key,
            bind_groups.texture_pool_samplers_bind_group_layout_key,
        ]);
        let singlesampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            &ctx.gpu,
            &ctx.bind_group_layouts,
            singlesampled_pipeline_layout_cache_key,
        )?;

        Ok(Self {
            multisampled_pipeline_layout_key,
            singlesampled_pipeline_layout_key,
            render_pipeline_keys: SecondaryMap::new(),
        })
    }

    pub async fn set_render_pipeline_key(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        mesh: &Mesh,
        mesh_key: MeshKey,
        shaders: &mut Shaders,
        pipelines: &mut Pipelines,
        material_bind_groups: &MaterialTransparentBindGroups,
        pipeline_layouts: &PipelineLayouts,
        mesh_buffer_infos: &MeshBufferInfos,
        anti_aliasing: &AntiAliasing,
        textures: &Textures,
        render_texture_formats: &RenderTextureFormats,
    ) -> Result<RenderPipelineKey> {
        let mesh_buffer_info = mesh_buffer_infos.get(mesh.buffer_info_key)?;

        let shader_cache_key = ShaderCacheKeyMaterialTransparent {
            attributes: mesh_buffer_info.into(),
            texture_pool_arrays_len: material_bind_groups.texture_pool_arrays_len,
            texture_pool_samplers_len: material_bind_groups.texture_pool_sampler_keys.len() as u32,
            msaa_sample_count: anti_aliasing.msaa_sample_count,
            mipmaps: anti_aliasing.mipmap,
            instancing_transforms: mesh.instanced,
        };

        let shader_key = shaders
            .get_key(
                gpu,
                ShaderCacheKeyMaterial::Transparent(shader_cache_key.clone()),
            )
            .await?;

        let color_targets = &[ColorTargetState::new(render_texture_formats.oit_color)];

        let render_pipeline_key = geometry_and_transparency_render_pipeline_key(
            gpu,
            shaders,
            pipelines,
            pipeline_layouts,
            render_texture_formats.depth,
            if anti_aliasing.msaa_sample_count.unwrap_or_default() > 0 {
                self.multisampled_pipeline_layout_key
            } else {
                self.singlesampled_pipeline_layout_key
            },
            shader_key,
            color_targets,
            false,
            anti_aliasing.msaa_sample_count,
            mesh.instanced,
            if mesh.double_sided {
                CullMode::None
            } else {
                CullMode::Back
            },
        )
        .await?;

        self.render_pipeline_keys
            .insert(mesh_key, render_pipeline_key.clone());

        Ok(render_pipeline_key)
    }

    pub fn get_render_pipeline_key(&self, mesh_key: MeshKey) -> Option<RenderPipelineKey> {
        self.render_pipeline_keys.get(mesh_key).cloned()
    }
}
