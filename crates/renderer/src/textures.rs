use std::collections::HashMap;

use awsm_renderer_core::{
    compare::CompareFunction,
    renderer::AwsmRendererWebGpu,
    sampler::{AddressMode, FilterMode, MipmapFilterMode, SamplerDescriptor},
};
use ordered_float::OrderedFloat;
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

pub struct Textures {
    textures: SlotMap<TextureKey, web_sys::GpuTexture>,
    samplers: SlotMap<SamplerKey, web_sys::GpuSampler>,
    sampler_cache: HashMap<SamplerCacheKey, SamplerKey>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SamplerCacheKey {
    pub address_mode_u: Option<AddressMode>,
    pub address_mode_v: Option<AddressMode>,
    pub address_mode_w: Option<AddressMode>,
    pub compare: Option<CompareFunction>,
    pub lod_min_clamp: Option<OrderedFloat<f32>>,
    pub lod_max_clamp: Option<OrderedFloat<f32>>,
    pub max_anisotropy: Option<u16>,
    pub mag_filter: Option<FilterMode>,
    pub min_filter: Option<FilterMode>,
    pub mipmap_filter: Option<MipmapFilterMode>,
}

impl std::hash::Hash for SamplerCacheKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.address_mode_u.map(|x| x as u32).hash(state);
        self.address_mode_v.map(|x| x as u32).hash(state);
        self.address_mode_w.map(|x| x as u32).hash(state);
        self.compare.map(|x| x as u32).hash(state);
        self.lod_min_clamp.hash(state);
        self.lod_max_clamp.hash(state);
        self.max_anisotropy.hash(state);
        self.mag_filter.map(|x| x as u32).hash(state);
        self.min_filter.map(|x| x as u32).hash(state);
        self.mipmap_filter.map(|x| x as u32).hash(state);
    }
}

impl Default for Textures {
    fn default() -> Self {
        Self::new()
    }
}

impl Textures {
    pub fn new() -> Self {
        Self {
            textures: SlotMap::with_key(),
            samplers: SlotMap::with_key(),
            sampler_cache: HashMap::new(),
        }
    }

    pub fn add_texture(&mut self, texture: web_sys::GpuTexture) -> TextureKey {
        self.textures.insert(texture)
    }

    pub fn get_texture(&self, key: TextureKey) -> Result<&web_sys::GpuTexture> {
        self.textures
            .get(key)
            .ok_or(AwsmTextureError::TextureNotFound(key))
    }

    pub fn remove_texture(&mut self, key: TextureKey) {
        if let Some(texture) = self.textures.remove(key) {
            texture.destroy();
        }
    }

    pub fn get_sampler_key(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        cache_key: SamplerCacheKey,
    ) -> Result<SamplerKey> {
        if let Some(sampler_key) = self.sampler_cache.get(&cache_key) {
            return Ok(*sampler_key);
        }

        let descriptor = SamplerDescriptor {
            label: None,
            address_mode_u: cache_key.address_mode_u,
            address_mode_v: cache_key.address_mode_v,
            address_mode_w: cache_key.address_mode_w,
            compare: cache_key.compare,
            lod_min_clamp: cache_key.lod_min_clamp.map(|x| x.into_inner()),
            lod_max_clamp: cache_key.lod_max_clamp.map(|x| x.into_inner()),
            max_anisotropy: cache_key.max_anisotropy,
            mag_filter: cache_key.mag_filter,
            min_filter: cache_key.min_filter,
            mipmap_filter: cache_key.mipmap_filter,
        };

        let sampler = gpu.create_sampler(Some(&descriptor.into()));

        let key = self.samplers.insert(sampler);
        self.sampler_cache.insert(cache_key, key);

        Ok(key)
    }

    pub fn get_sampler(&self, key: SamplerKey) -> Result<&web_sys::GpuSampler> {
        self.samplers
            .get(key)
            .ok_or(AwsmTextureError::SamplerNotFound(key))
    }

    pub fn remove_sampler(&mut self, key: SamplerKey) {
        self.samplers.remove(key);
    }
}

new_key_type! {
    pub struct TextureKey;
}

new_key_type! {
    pub struct SamplerKey;
}

pub type Result<T> = std::result::Result<T, AwsmTextureError>;

#[derive(Error, Debug)]
pub enum AwsmTextureError {
    #[error("[shader] sampler not found: {0:?}")]
    SamplerNotFound(SamplerKey),

    #[error("[shader] texture not found: {0:?}")]
    TextureNotFound(TextureKey),
}
