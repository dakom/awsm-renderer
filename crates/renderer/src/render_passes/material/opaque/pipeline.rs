use crate::error::Result;
use crate::pipeline_layouts::PipelineLayoutCacheKey;
use crate::pipelines::compute_pipeline::{ComputePipelineCacheKey, ComputePipelineKey};
use crate::render_passes::material::cache_key::ShaderCacheKeyMaterial;
use crate::render_passes::material::opaque::shader::cache_key::ShaderCacheKeyMaterialOpaque;
use crate::render_passes::{
    material::opaque::bind_group::MaterialOpaqueBindGroups, RenderPassInitContext,
};

pub struct MaterialOpaquePipelines {
    pub compute_pipeline_key: ComputePipelineKey,
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

        let shader_key = ctx
            .shaders
            .get_key(
                &ctx.gpu,
                ShaderCacheKeyMaterial::Opaque(ShaderCacheKeyMaterialOpaque {
                    texture_bindings: bind_groups.texture_bindings.clone(),
                }),
            )
            .await?;

        let compute_pipeline_cache_key =
            ComputePipelineCacheKey::new(shader_key, pipeline_layout_key);

        let compute_pipeline_key = ctx
            .pipelines
            .compute
            .get_key(
                &ctx.gpu,
                &ctx.shaders,
                &ctx.pipeline_layouts,
                compute_pipeline_cache_key,
            )
            .await?;

        Ok(Self {
            compute_pipeline_key,
        })
    }
}
