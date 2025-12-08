use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::SecondaryMap;

use crate::anti_alias::AntiAliasing;
use crate::error::Result;
use crate::mesh::{Mesh, MeshBufferInfos, MeshKey};
use crate::pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey, PipelineLayouts};
use crate::pipelines::compute_pipeline::{ComputePipelineCacheKey, ComputePipelineKey};
use crate::pipelines::Pipelines;
use crate::render_passes::{
    material_opaque::{
        bind_group::MaterialOpaqueBindGroups, shader::cache_key::ShaderCacheKeyMaterialOpaque,
    },
    RenderPassInitContext,
};
use crate::shaders::Shaders;
use crate::textures::Textures;

pub struct MaterialOpaquePipelines {
    multisampled_pipeline_layout_key: PipelineLayoutKey,
    singlesampled_pipeline_layout_key: PipelineLayoutKey,
    compute_pipeline_keys: SecondaryMap<MeshKey, ComputePipelineKey>,
}

impl MaterialOpaquePipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &MaterialOpaqueBindGroups,
    ) -> Result<Self> {
        let multisampled_pipeline_layout_cache_key = PipelineLayoutCacheKey::new(vec![
            bind_groups.multisampled_main_bind_group_layout_key,
            bind_groups.lights_bind_group_layout_key,
            bind_groups.texture_pool_textures_bind_group_layout_key,
        ]);
        let multisampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            ctx.gpu,
            ctx.bind_group_layouts,
            multisampled_pipeline_layout_cache_key,
        )?;

        let singlesampled_pipeline_layout_cache_key = PipelineLayoutCacheKey::new(vec![
            bind_groups.singlesampled_main_bind_group_layout_key,
            bind_groups.lights_bind_group_layout_key,
            bind_groups.texture_pool_textures_bind_group_layout_key,
        ]);
        let singlesampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            ctx.gpu,
            ctx.bind_group_layouts,
            singlesampled_pipeline_layout_cache_key,
        )?;

        Ok(Self {
            multisampled_pipeline_layout_key,
            singlesampled_pipeline_layout_key,
            compute_pipeline_keys: SecondaryMap::new(),
        })
    }

    pub fn get_compute_pipeline_key(&self, mesh_key: MeshKey) -> Option<ComputePipelineKey> {
        self.compute_pipeline_keys.get(mesh_key).cloned()
    }

    pub async fn set_compute_pipeline_key(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        mesh: &Mesh,
        mesh_key: MeshKey,
        shaders: &mut Shaders,
        pipelines: &mut Pipelines,
        material_bind_groups: &MaterialOpaqueBindGroups,
        pipeline_layouts: &PipelineLayouts,
        mesh_buffer_infos: &MeshBufferInfos,
        anti_aliasing: &AntiAliasing,
        _textures: &Textures,
    ) -> Result<ComputePipelineKey> {
        let mesh_buffer_info = mesh_buffer_infos.get(mesh.buffer_info_key)?;

        let shader_cache_key = ShaderCacheKeyMaterialOpaque {
            attributes: mesh_buffer_info.into(),
            texture_pool_arrays_len: material_bind_groups.texture_pool_arrays_len,
            texture_pool_samplers_len: material_bind_groups.texture_pool_sampler_keys.len() as u32,
            msaa_sample_count: anti_aliasing.msaa_sample_count,
            mipmaps: anti_aliasing.mipmap,
        };

        let shader_key = shaders.get_key(gpu, shader_cache_key).await?;

        let compute_pipeline_cache_key = ComputePipelineCacheKey::new(
            shader_key,
            if anti_aliasing.msaa_sample_count.is_some() {
                self.multisampled_pipeline_layout_key
            } else {
                self.singlesampled_pipeline_layout_key
            },
        );

        let compute_pipeline_key = pipelines
            .compute
            .get_key(
                gpu,
                shaders,
                pipeline_layouts,
                compute_pipeline_cache_key.clone(),
            )
            .await?;

        self.compute_pipeline_keys
            .insert(mesh_key, compute_pipeline_key);

        Ok(compute_pipeline_key)
    }
}
