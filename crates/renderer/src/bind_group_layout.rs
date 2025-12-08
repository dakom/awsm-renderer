use std::collections::HashMap;

use awsm_renderer_core::{
    bind_groups::{BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupLayoutResource},
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

pub struct BindGroupLayouts {
    lookup: SlotMap<BindGroupLayoutKey, web_sys::GpuBindGroupLayout>,
    cache: HashMap<BindGroupLayoutCacheKey, BindGroupLayoutKey>,
}

impl BindGroupLayouts {
    pub fn new() -> Self {
        Self {
            lookup: SlotMap::with_key(),
            cache: HashMap::new(),
        }
    }

    pub fn get_key(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        cache_key: BindGroupLayoutCacheKey,
    ) -> Result<BindGroupLayoutKey> {
        if let Some(key) = self.cache.get(&cache_key) {
            return Ok(*key);
        }

        let entries = cache_key
            .entries
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, entry)| BindGroupLayoutEntry {
                binding: index as u32,
                visibility_compute: entry.visibility_compute,
                visibility_vertex: entry.visibility_vertex,
                visibility_fragment: entry.visibility_fragment,
                resource: entry.resource,
            })
            .collect();

        let bind_group_layout = gpu
            .create_bind_group_layout(
                &BindGroupLayoutDescriptor::new(None)
                    .with_entries(entries)
                    .into(),
            )
            .map_err(AwsmBindGroupLayoutError::Create)?;

        let key = self.lookup.insert(bind_group_layout);
        self.cache.insert(cache_key, key);
        Ok(key)
    }

    pub fn get(&self, key: BindGroupLayoutKey) -> Result<&web_sys::GpuBindGroupLayout> {
        self.lookup
            .get(key)
            .ok_or(AwsmBindGroupLayoutError::NotFound(key))
    }
}

impl Default for BindGroupLayouts {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct BindGroupLayoutCacheKey {
    pub entries: Vec<BindGroupLayoutCacheKeyEntry>,
}
impl BindGroupLayoutCacheKey {
    pub fn new(entries: Vec<BindGroupLayoutCacheKeyEntry>) -> Self {
        Self { entries }
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct BindGroupLayoutCacheKeyEntry {
    pub resource: BindGroupLayoutResource,
    pub visibility_compute: bool,
    pub visibility_vertex: bool,
    pub visibility_fragment: bool,
}

new_key_type! {
    pub struct BindGroupLayoutKey;
}

type Result<T> = std::result::Result<T, AwsmBindGroupLayoutError>;
#[derive(Error, Debug)]
pub enum AwsmBindGroupLayoutError {
    #[error("[bind group layout] Unable to create: {0:?}")]
    Create(AwsmCoreError),

    #[error("[bind group layout] Not found: {0:?}")]
    NotFound(BindGroupLayoutKey),
}
