use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::SecondaryMap;

use crate::bind_groups::BindGroups;
use crate::error::Result;
use crate::materials::MaterialKey;
use crate::mesh::{MeshBufferInfo, MeshBufferInfoKey, MeshBufferInfos};
use crate::pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey, PipelineLayouts};
use crate::pipelines::compute_pipeline::{ComputePipelineCacheKey, ComputePipelineKey};
use crate::pipelines::Pipelines;
use crate::render_passes::material::cache_key::ShaderCacheKeyMaterial;
use crate::render_passes::material::opaque::shader::cache_key::ShaderCacheKeyMaterialOpaque;
use crate::render_passes::{
    material::opaque::bind_group::MaterialOpaqueBindGroups, RenderPassInitContext,
};
use crate::shaders::Shaders;

pub struct MaterialOpaquePipelines {
    pipeline_layout_key: PipelineLayoutKey,
    compute_pipeline_keys:
        SecondaryMap<MeshBufferInfoKey, SecondaryMap<MaterialKey, ComputePipelineKey>>,
}

impl MaterialOpaquePipelines {
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &MaterialOpaqueBindGroups,
    ) -> Result<Self> {
        let pipeline_layout_cache_key =
            PipelineLayoutCacheKey::new(bind_groups.bind_group_layout_keys.clone());
        let pipeline_layout_key = ctx.pipeline_layouts.get_key(
            &ctx.gpu,
            &ctx.bind_group_layouts,
            pipeline_layout_cache_key,
        )?;

        Ok(Self {
            pipeline_layout_key,
            compute_pipeline_keys: SecondaryMap::new(),
        })
    }

    pub fn get_compute_pipeline_key(
        &self,
        mesh_buffer_info_key: MeshBufferInfoKey,
        material_key: MaterialKey,
    ) -> Option<ComputePipelineKey> {
        self.compute_pipeline_keys
            .get(mesh_buffer_info_key)
            .and_then(|m| m.get(material_key))
            .copied()
    }

    pub async fn set_compute_pipeline_key(
        &mut self,
        mesh_buffer_info_key: MeshBufferInfoKey,
        material_key: MaterialKey,
        gpu: &AwsmRendererWebGpu,
        shaders: &mut Shaders,
        pipelines: &mut Pipelines,
        material_opaque_bind_groups: &MaterialOpaqueBindGroups,
        pipeline_layouts: &PipelineLayouts,
        mesh_buffer_infos: &MeshBufferInfos,
    ) -> Result<ComputePipelineKey> {
        let mesh_buffer_info = mesh_buffer_infos.get(mesh_buffer_info_key)?;

        let shader_cache_key = ShaderCacheKeyMaterialOpaque {
            attributes: mesh_buffer_info.into(),
            texture_bindings: material_opaque_bind_groups.texture_bindings.clone(),
            sampler_bindings: material_opaque_bind_groups.sampler_bindings.clone(),
        };

        let shader_key = shaders
            .get_key(
                gpu,
                ShaderCacheKeyMaterial::Opaque(shader_cache_key.clone()),
            )
            .await?;

        let compute_pipeline_cache_key =
            ComputePipelineCacheKey::new(shader_key, self.pipeline_layout_key);

        let compute_pipeline_key = pipelines
            .compute
            .get_key(
                &gpu,
                &shaders,
                &pipeline_layouts,
                compute_pipeline_cache_key.clone(),
            )
            .await?;

        match self.compute_pipeline_keys.entry(mesh_buffer_info_key) {
            None => {
                // this isn't "if the key doesn't exist yet"
                // it's "if the key was removed"
                let mut m = SecondaryMap::new();
                m.insert(material_key, compute_pipeline_key);
                self.compute_pipeline_keys.insert(mesh_buffer_info_key, m);
            }
            Some(x) => {
                x.or_insert_with(SecondaryMap::new)
                    .insert(material_key, compute_pipeline_key);
            }
        }

        Ok(compute_pipeline_key)
    }
}
