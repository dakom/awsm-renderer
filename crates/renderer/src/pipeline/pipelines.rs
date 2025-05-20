use std::collections::HashMap;

use awsm_renderer_core::{error::AwsmCoreError, renderer::AwsmRendererWebGpu};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::{AwsmBindGroupError, BindGroups},
    shaders::{ShaderKey, Shaders},
};

use super::{cache::PipelineLayoutCacheKey, RenderPipelineCacheKey};

pub struct Pipelines {
    render_pipeline: SlotMap<RenderPipelineKey, web_sys::GpuRenderPipeline>,
    layout: SlotMap<PipelineLayoutKey, web_sys::GpuPipelineLayout>,
    render_pipeline_cache: HashMap<RenderPipelineCacheKey, RenderPipelineKey>,
    layout_cache: HashMap<PipelineLayoutCacheKey, PipelineLayoutKey>,
}

impl Pipelines {
    pub fn new() -> Self {
        Self {
            render_pipeline: SlotMap::with_key(),
            layout: SlotMap::with_key(),
            render_pipeline_cache: HashMap::new(),
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

    pub fn add_pipeline_layout(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &BindGroups,
        label: Option<&str>,
        cache_key: PipelineLayoutCacheKey,
    ) -> Result<PipelineLayoutKey> {
        if let Some(layout_key) = self.get_pipeline_layout_key_from_cache(&cache_key) {
            return Ok(layout_key);
        }

        let layout = gpu.create_pipeline_layout(
            &cache_key
                .clone()
                .into_descriptor(bind_groups, label)?
                .into(),
        );

        let layout_key = self.layout.insert(layout);

        self.layout_cache.insert(cache_key, layout_key);

        Ok(layout_key)
    }

    pub async fn add_render_pipeline(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        shaders: &Shaders,
        label: Option<&str>,
        cache_key: RenderPipelineCacheKey,
    ) -> Result<RenderPipelineKey> {
        if let Some(pipeline_key) = self.get_render_pipeline_key_from_cache(&cache_key) {
            return Ok(pipeline_key);
        }

        let layout = self
            .layout
            .get(cache_key.layout_key)
            .ok_or_else(|| AwsmPipelineError::MissingPipelineLayout(cache_key.layout_key))?;

        let shader = shaders
            .get_shader(cache_key.shader_key)
            .ok_or_else(|| AwsmPipelineError::MissingShader(cache_key.shader_key))?;

        let pipeline = gpu
            .create_render_pipeline(
                &cache_key
                    .clone()
                    .into_descriptor(shader, layout, label)?
                    .into(),
            )
            .await?;

        let pipeline_key = self.render_pipeline.insert(pipeline);

        self.render_pipeline_cache.insert(cache_key, pipeline_key);

        Ok(pipeline_key)
    }

    pub fn get_pipeline_layout_key_from_cache(
        &mut self,
        key: &PipelineLayoutCacheKey,
    ) -> Option<PipelineLayoutKey> {
        self.layout_cache.get(key).cloned()
    }

    pub fn get_render_pipeline_key_from_cache(
        &mut self,
        key: &RenderPipelineCacheKey,
    ) -> Option<RenderPipelineKey> {
        self.render_pipeline_cache.get(key).cloned()
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
