//! Effects pass pipeline setup.

use awsm_renderer_core::renderer::AwsmRendererWebGpu;

use crate::{
    anti_alias::AntiAliasing,
    error::Result,
    pipeline_layouts::{PipelineLayoutCacheKey, PipelineLayoutKey, PipelineLayouts},
    pipelines::{
        compute_pipeline::{ComputePipelineCacheKey, ComputePipelineKey},
        Pipelines,
    },
    post_process::PostProcessing,
    render_passes::{
        effects::{
            bind_group::EffectsBindGroups,
            shader::cache_key::{BloomPhase, ShaderCacheKeyEffects},
        },
        RenderPassInitContext,
    },
    render_textures::RenderTextureFormats,
    shaders::Shaders,
};

/// Number of bloom blur passes (more = smoother but slower).
/// Total passes = 1 extract + BLOOM_BLUR_PASSES + 1 blend.
pub const BLOOM_BLUR_PASSES: u32 = 3;

/// Compute pipelines for post-processing effects.
pub struct EffectsPipelines {
    multisampled_pipeline_layout_key: PipelineLayoutKey,
    singlesampled_pipeline_layout_key: PipelineLayoutKey,

    // When bloom is disabled - single pass for other effects
    no_bloom_pipeline: Option<ComputePipelineKey>,

    // When bloom is enabled - multi-pass pipelines
    bloom_extract_pipeline: Option<ComputePipelineKey>, // Always ping_pong=false
    bloom_blur_pipeline_a: Option<ComputePipelineKey>,  // ping_pong=false
    bloom_blur_pipeline_b: Option<ComputePipelineKey>,  // ping_pong=true
    bloom_blend_pipeline: Option<ComputePipelineKey>, // Always ping_pong=false (to write to effects_tex)
}

impl EffectsPipelines {
    /// Creates pipeline layout state for the effects pass.
    pub async fn new(
        ctx: &mut RenderPassInitContext<'_>,
        bind_groups: &EffectsBindGroups,
    ) -> Result<Self> {
        let singlesampled_pipeline_layout_cache_key =
            PipelineLayoutCacheKey::new(vec![bind_groups.singlesampled_bind_group_layout_key]);
        let multisampled_pipeline_layout_cache_key =
            PipelineLayoutCacheKey::new(vec![bind_groups.multisampled_bind_group_layout_key]);

        let singlesampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            ctx.gpu,
            ctx.bind_group_layouts,
            singlesampled_pipeline_layout_cache_key,
        )?;

        let multisampled_pipeline_layout_key = ctx.pipeline_layouts.get_key(
            ctx.gpu,
            ctx.bind_group_layouts,
            multisampled_pipeline_layout_cache_key,
        )?;

        Ok(Self {
            multisampled_pipeline_layout_key,
            singlesampled_pipeline_layout_key,
            no_bloom_pipeline: None,
            bloom_extract_pipeline: None,
            bloom_blur_pipeline_a: None,
            bloom_blur_pipeline_b: None,
            bloom_blend_pipeline: None,
        })
    }

    /// Get pipeline for a specific bloom phase and ping_pong state
    pub fn get_bloom_pipeline(
        &self,
        phase: BloomPhase,
        ping_pong: bool,
    ) -> Option<ComputePipelineKey> {
        match phase {
            BloomPhase::None => self.no_bloom_pipeline,
            BloomPhase::Extract => self.bloom_extract_pipeline,
            BloomPhase::Blur => {
                if ping_pong {
                    self.bloom_blur_pipeline_b
                } else {
                    self.bloom_blur_pipeline_a
                }
            }
            BloomPhase::Blend => self.bloom_blend_pipeline,
        }
    }

    /// Updates pipelines for the current anti-aliasing and post-processing settings.
    pub async fn set_render_pipeline_keys(
        &mut self,
        anti_aliasing: &AntiAliasing,
        post_processing: &PostProcessing,
        gpu: &AwsmRendererWebGpu,
        shaders: &mut Shaders,
        pipelines: &mut Pipelines,
        pipeline_layouts: &PipelineLayouts,
        render_texture_formats: &RenderTextureFormats,
    ) -> Result<()> {
        let multisampled_geometry = anti_aliasing.has_msaa_checked()?;

        // Always create no-bloom pipeline for when bloom is disabled
        self.no_bloom_pipeline = Some(
            self.create_pipeline(
                anti_aliasing,
                post_processing,
                gpu,
                shaders,
                pipelines,
                pipeline_layouts,
                render_texture_formats,
                BloomPhase::None,
                false,
                multisampled_geometry,
            )
            .await?,
        );

        // Create bloom pipelines if bloom might be used
        // Extract: always ping_pong=false (first pass writes to effects_tex)
        self.bloom_extract_pipeline = Some(
            self.create_pipeline(
                anti_aliasing,
                post_processing,
                gpu,
                shaders,
                pipelines,
                pipeline_layouts,
                render_texture_formats,
                BloomPhase::Extract,
                false,
                multisampled_geometry,
            )
            .await?,
        );

        // Blur: need both ping_pong variants for middle passes
        self.bloom_blur_pipeline_a = Some(
            self.create_pipeline(
                anti_aliasing,
                post_processing,
                gpu,
                shaders,
                pipelines,
                pipeline_layouts,
                render_texture_formats,
                BloomPhase::Blur,
                false,
                multisampled_geometry,
            )
            .await?,
        );

        self.bloom_blur_pipeline_b = Some(
            self.create_pipeline(
                anti_aliasing,
                post_processing,
                gpu,
                shaders,
                pipelines,
                pipeline_layouts,
                render_texture_formats,
                BloomPhase::Blur,
                true,
                multisampled_geometry,
            )
            .await?,
        );

        // Blend pass ping_pong is determined by total pass count to ensure final output goes to effects_tex
        let blend_ping_pong = (1 + BLOOM_BLUR_PASSES) % 2 == 1;

        self.bloom_blend_pipeline = Some(
            self.create_pipeline(
                anti_aliasing,
                post_processing,
                gpu,
                shaders,
                pipelines,
                pipeline_layouts,
                render_texture_formats,
                BloomPhase::Blend,
                blend_ping_pong,
                multisampled_geometry,
            )
            .await?,
        );

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_pipeline(
        &self,
        anti_aliasing: &AntiAliasing,
        _post_processing: &PostProcessing,
        gpu: &AwsmRendererWebGpu,
        shaders: &mut Shaders,
        pipelines: &mut Pipelines,
        pipeline_layouts: &PipelineLayouts,
        _render_texture_formats: &RenderTextureFormats,
        bloom_phase: BloomPhase,
        ping_pong: bool,
        multisampled_geometry: bool,
    ) -> Result<ComputePipelineKey> {
        let shader_cache_key = ShaderCacheKeyEffects {
            smaa_anti_alias: anti_aliasing.smaa,
            bloom_phase,
            dof: _post_processing.dof,
            ping_pong,
            multisampled_geometry,
        };
        let shader_key = shaders.get_key(gpu, shader_cache_key).await?;

        let compute_pipeline_cache_key = ComputePipelineCacheKey::new(
            shader_key,
            if multisampled_geometry {
                self.multisampled_pipeline_layout_key
            } else {
                self.singlesampled_pipeline_layout_key
            },
        );

        Ok(pipelines
            .compute
            .get_key(gpu, shaders, pipeline_layouts, compute_pipeline_cache_key)
            .await?)
    }
}
