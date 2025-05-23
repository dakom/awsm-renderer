use std::collections::HashMap;

use awsm_renderer_core::error::AwsmCoreError;
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{bind_groups::AwsmBindGroupError, shaders::ShaderKey, AwsmRenderer};

use super::{cache::PipelineLayoutCacheKey, RenderPipelineCacheKey};

pub struct Pipelines {
    render_pipeline: SlotMap<RenderPipelineKey, web_sys::GpuRenderPipeline>,
    layout: SlotMap<PipelineLayoutKey, web_sys::GpuPipelineLayout>,
    render_pipeline_cache: HashMap<RenderPipelineCacheKey, RenderPipelineKey>,
    reverse_render_pipeline_cache: HashMap<RenderPipelineKey, RenderPipelineCacheKey>,
    layout_cache: HashMap<PipelineLayoutCacheKey, PipelineLayoutKey>,
}

impl Default for Pipelines {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipelines {
    pub fn new() -> Self {
        Self {
            render_pipeline: SlotMap::with_key(),
            layout: SlotMap::with_key(),
            render_pipeline_cache: HashMap::new(),
            reverse_render_pipeline_cache: HashMap::new(),
            layout_cache: HashMap::new(),
        }
    }

    pub fn get_pipeline_layout(
        &self,
        key: PipelineLayoutKey,
    ) -> Result<&web_sys::GpuPipelineLayout> {
        self.layout
            .get(key)
            .ok_or(AwsmPipelineError::MissingPipelineLayout(key))
    }
    pub fn get_render_pipeline(
        &self,
        key: RenderPipelineKey,
    ) -> Result<&web_sys::GpuRenderPipeline> {
        self.render_pipeline
            .get(key)
            .ok_or(AwsmPipelineError::MissingRenderPipeline(key))
    }

    pub fn get_pipeline_layout_key_from_cache(
        &self,
        key: &PipelineLayoutCacheKey,
    ) -> Option<PipelineLayoutKey> {
        self.layout_cache.get(key).cloned()
    }

    pub fn get_render_pipeline_key_from_cache(
        &self,
        key: &RenderPipelineCacheKey,
    ) -> Option<RenderPipelineKey> {
        self.render_pipeline_cache.get(key).cloned()
    }

    pub fn get_render_pipeline_cache_from_key(
        &self,
        key: &RenderPipelineKey,
    ) -> Option<RenderPipelineCacheKey> {
        self.reverse_render_pipeline_cache.get(key).cloned()
    }
}

impl AwsmRenderer {
    pub fn add_pipeline_layout(
        &mut self,
        label: Option<&str>,
        cache_key: PipelineLayoutCacheKey,
    ) -> Result<PipelineLayoutKey> {
        if let Some(layout_key) = self
            .pipelines
            .get_pipeline_layout_key_from_cache(&cache_key)
        {
            return Ok(layout_key);
        }

        let layout = self.gpu.create_pipeline_layout(
            &cache_key
                .clone()
                .into_descriptor(&self.bind_groups, label)?
                .into(),
        );

        let layout_key = self.pipelines.layout.insert(layout);

        self.pipelines.layout_cache.insert(cache_key, layout_key);

        Ok(layout_key)
    }

    pub async fn add_render_pipeline(
        &mut self,
        label: Option<&str>,
        cache_key: RenderPipelineCacheKey,
    ) -> Result<RenderPipelineKey> {
        if let Some(pipeline_key) = self
            .pipelines
            .get_render_pipeline_key_from_cache(&cache_key)
        {
            return Ok(pipeline_key);
        }

        let layout = self.pipelines.layout.get(cache_key.layout_key).ok_or(
            AwsmPipelineError::MissingPipelineLayout(cache_key.layout_key),
        )?;

        let shader = self
            .shaders
            .get_shader(cache_key.shader_key)
            .ok_or(AwsmPipelineError::MissingShader(cache_key.shader_key))?;

        let pipeline = self
            .gpu
            .create_render_pipeline(&cache_key.clone().into_descriptor(shader, layout, label)?)
            .await?;

        let pipeline_key = self.pipelines.render_pipeline.insert(pipeline);

        self.pipelines
            .render_pipeline_cache
            .insert(cache_key.clone(), pipeline_key);
        self.pipelines
            .reverse_render_pipeline_cache
            .insert(pipeline_key, cache_key);

        Ok(pipeline_key)
    }
}

new_key_type! {
    pub struct RenderPipelineKey;
}

new_key_type! {
    pub struct PipelineLayoutKey;
}

type Result<T> = std::result::Result<T, AwsmPipelineError>;

#[derive(Error, Debug)]
pub enum AwsmPipelineError {
    #[error("[render pipeline] missing pipeline: {0:?}")]
    MissingRenderPipeline(RenderPipelineKey),

    #[error("[render pipeline] missing shader: {0:?}")]
    MissingShader(ShaderKey),

    #[error("[render pipeline] missing layout: {0:?}")]
    MissingPipelineLayout(PipelineLayoutKey),

    #[error("[render pipeline] bind group: {0:?}")]
    BindGroup(#[from] AwsmBindGroupError),

    #[error("[render pipeline]: {0:?}")]
    Core(#[from] AwsmCoreError),
}
