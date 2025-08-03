use crate::pipeline_layouts::PipelineLayoutCacheKey;
use crate::pipelines::compute_pipeline::{ComputePipelineCacheKey, ComputePipelineKey};
use crate::render_passes::material::looks::pbr::shader::cache_key::ShaderCacheKeyMaterialPbr;
use crate::render_passes::material::looks::shader_cache_key::ShaderCacheKeyMaterialLook;
use crate::render_passes::material::opaque::shader::cache_key::ShaderCacheKeyMaterialOpaque;
use crate::render_passes::{material::opaque::bind_group::MaterialOpaqueBindGroups, RenderPassInitContext};
use crate::error::Result;

pub struct MaterialOpaquePipelines {
    pub compute_pipeline_key: ComputePipelineKey
}

impl MaterialOpaquePipelines {
    pub async fn new(ctx: &mut RenderPassInitContext, bind_groups: &MaterialOpaqueBindGroups) -> Result<Self> {
        let pipeline_layout_cache_key = PipelineLayoutCacheKey::new(vec![bind_groups.bind_group_layout_key.clone()]);
        let pipeline_layout_key = ctx.pipeline_layouts.get_key(&ctx.gpu, &ctx.bind_group_layouts, pipeline_layout_cache_key)?;

        let shader_cache_key = ShaderCacheKeyMaterialOpaque{
            look: ShaderCacheKeyMaterialLook::Pbr(
                ShaderCacheKeyMaterialPbr::default()
            )
        };
        let shader_key = ctx.shaders.get_key(&ctx.gpu, shader_cache_key).await?;

        let compute_pipeline_cache_key = ComputePipelineCacheKey::new(shader_key, pipeline_layout_key);

        let compute_pipeline_key = ctx.pipelines.compute.get_key(&ctx.gpu, &ctx.shaders, &ctx.pipeline_layouts, compute_pipeline_cache_key).await?;


        Ok(Self {
            compute_pipeline_key
        })
    }
}