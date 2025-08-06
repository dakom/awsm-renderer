use std::collections::HashMap;

use awsm_renderer_core::{
    compare::CompareFunction,
    error::AwsmCoreError,
    image::ImageData,
    renderer::AwsmRendererWebGpu,
    sampler::{AddressMode, FilterMode, MipmapFilterMode, SamplerDescriptor},
    texture::mega_texture::{self, MegaTexture, MegaTextureEntry, MegaTextureEntryInfo},
};
use ordered_float::OrderedFloat;
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::{BindGroupCreate, BindGroups},
    AwsmRendererLogging,
};

pub struct Textures {
    pub texture_arrays: Vec<web_sys::GpuTexture>,
    pub mega_texture: MegaTexture<TextureKey>,
    textures: SlotMap<TextureKey, MegaTextureEntryInfo<TextureKey>>,
    samplers: SlotMap<SamplerKey, web_sys::GpuSampler>,
    sampler_cache: HashMap<SamplerCacheKey, SamplerKey>,
    gpu_dirty: bool,
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

impl Textures {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Self {
        Self {
            texture_arrays: Vec::new(),
            mega_texture: MegaTexture::new(&gpu.device.limits(), 8),
            textures: SlotMap::with_key(),
            samplers: SlotMap::with_key(),
            sampler_cache: HashMap::new(),
            gpu_dirty: false,
        }
    }

    pub fn add_image(&mut self, image_data: ImageData) -> Result<TextureKey> {
        let key = self.textures.try_insert_with_key(|key| {
            self.mega_texture
                .add_entries(vec![(image_data, key)])
                .map_err(AwsmTextureError::from)
                .and_then(|mut entries| entries.pop().ok_or(AwsmTextureError::MegaTexture))
        })?;

        self.gpu_dirty = true;

        Ok(key)
    }

    pub async fn write_gpu_textures(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Textures GPU write").entered())
            } else {
                None
            };

            // TODO - only need to write _new_ arrays, not all of them...
            self.texture_arrays = self.mega_texture.write_texture_arrays(gpu).await?;

            bind_groups.mark_create(BindGroupCreate::MegaTexture);

            self.gpu_dirty = false;
        }

        Ok(())
    }

    pub fn get_entry(&self, key: TextureKey) -> Result<&MegaTextureEntryInfo<TextureKey>> {
        self.textures
            .get(key)
            .ok_or(AwsmTextureError::TextureNotFound(key))
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
    #[error("[texture] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[texture] mega-texture failure")]
    MegaTexture,

    #[error("[texture] sampler not found: {0:?}")]
    SamplerNotFound(SamplerKey),

    #[error("[texture] texture not found: {0:?}")]
    TextureNotFound(TextureKey),
}
