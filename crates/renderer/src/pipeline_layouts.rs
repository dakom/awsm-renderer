use std::collections::HashMap;

use awsm_renderer_core::{error::AwsmCoreError, pipeline::layout::PipelineLayoutDescriptor, renderer::AwsmRendererWebGpu};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::bind_group_layout::{AwsmBindGroupLayoutError, BindGroupLayoutKey, BindGroupLayouts};

pub struct PipelineLayouts {
    lookup: SlotMap<PipelineLayoutKey, web_sys::GpuPipelineLayout>,
    cache: HashMap<PipelineLayoutCacheKey, PipelineLayoutKey>,
}

impl PipelineLayouts {
    pub fn new() -> Self {
        Self {
            lookup: SlotMap::with_key(),
            cache: HashMap::new(),
        }
    }

    pub fn get_key(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        bind_group_layouts: &BindGroupLayouts,
        cache_key: PipelineLayoutCacheKey,
    ) -> Result<PipelineLayoutKey> {
        if let Some(key) = self.cache.get(&cache_key) {
            return Ok(*key);
        }

        let bind_group_layout_entries = cache_key
            .bind_group_layouts
            .iter()
            .map(|key| {
                bind_group_layouts
                    .get(*key)
                    .map(|layout| layout.clone())
                    .map_err(AwsmPipelineLayoutError::BindGroupLayout)
            })
            .collect::<Result<Vec<_>>>()?;

        let pipeline_layout = gpu.create_pipeline_layout(&PipelineLayoutDescriptor::new(None, bind_group_layout_entries).into());

        let key = self.lookup.insert(pipeline_layout);
        self.cache.insert(cache_key, key);
        Ok(key)
    }

    pub fn get(
        &self,
        key: PipelineLayoutKey,
    ) -> Result<&web_sys::GpuPipelineLayout> {
        self.lookup.get(key).ok_or(AwsmPipelineLayoutError::NotFound(key))
    }
}

impl Default for PipelineLayouts {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineLayoutCacheKey {
    bind_group_layouts: Vec<BindGroupLayoutKey>,
}

impl PipelineLayoutCacheKey {
    pub fn new(bind_group_layouts: Vec<BindGroupLayoutKey>) -> Self {
        Self { bind_group_layouts }
    }
}


new_key_type! {
    pub struct PipelineLayoutKey;
}

type Result<T> = std::result::Result<T, AwsmPipelineLayoutError>;

#[derive(Error, Debug)]
pub enum AwsmPipelineLayoutError {
    #[error("[pipeline layout] Unable to create: {0:?}")]
    Create(AwsmCoreError),

    #[error("[pipeline layout] Not found: {0:?}")]
    NotFound(PipelineLayoutKey),

    #[error("[pipeline layout] {0:?}")]
    BindGroupLayout(#[from] AwsmBindGroupLayoutError)
}
