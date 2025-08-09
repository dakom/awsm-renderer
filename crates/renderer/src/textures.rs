use std::collections::HashMap;

use awsm_renderer_core::{
    compare::CompareFunction,
    error::AwsmCoreError,
    image::ImageData,
    renderer::AwsmRendererWebGpu,
    sampler::{AddressMode, FilterMode, MipmapFilterMode, SamplerDescriptor},
    texture::{
        mega_texture::{self, MegaTexture, MegaTextureEntry, MegaTextureEntryInfo},
        TextureViewDescriptor,
    },
};
use ordered_float::OrderedFloat;
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;
use web_sys::GpuSupportedLimits;

use crate::{
    bind_groups::{BindGroupCreate, BindGroups},
    error::AwsmError,
    render_passes::RenderPassInitContext,
    AwsmRenderer, AwsmRendererLogging,
};

impl AwsmRenderer {
    // this should ideally only be called after all the textures have been loaded
    pub async fn finalize_gpu_textures(&mut self) -> std::result::Result<(), AwsmError> {
        let was_dirty = self
            .textures
            .write_gpu_textures(&self.logging, &self.gpu)
            .await?;

        if was_dirty {
            let mut render_pass_ctx = RenderPassInitContext {
                gpu: &mut self.gpu,
                pipelines: &mut self.pipelines,
                shaders: &mut self.shaders,
                textures: &mut self.textures,
                render_texture_formats: &mut self.render_textures.formats,
                bind_group_layouts: &mut self.bind_group_layouts,
                pipeline_layouts: &mut self.pipeline_layouts,
            };

            self.render_passes
                .update_texture_bindings(&mut render_pass_ctx)
                .await?;

            self.bind_groups.mark_create(BindGroupCreate::MegaTexture);

            self.textures
                .mega_texture
                .size_report(&self.gpu.device.limits())
                .console_log();
        }
        Ok(())
    }
}

pub struct Textures {
    pub gpu_texture_arrays: Vec<web_sys::GpuTexture>,
    pub gpu_texture_array_views: Vec<web_sys::GpuTextureView>,
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
            gpu_texture_arrays: Vec::new(),
            gpu_texture_array_views: Vec::new(),
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

    async fn write_gpu_textures(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
    ) -> Result<bool> {
        let was_gpu_dirty = self.gpu_dirty;
        if self.gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Textures GPU write").entered())
            } else {
                None
            };

            // TODO - only need to write _new_ arrays, not all of them...
            self.gpu_texture_arrays = self.mega_texture.write_texture_arrays(gpu).await?;
            self.gpu_texture_array_views = self
                .gpu_texture_arrays
                .iter()
                .enumerate()
                .map(|(index, texture)| {
                    let descriptor = TextureViewDescriptor::new(Some("Mega Texture View"))
                        .with_dimension(web_sys::GpuTextureViewDimension::N2dArray)
                        .with_array_layer_count(self.mega_texture.layer_len(index) as u32)
                        .with_mip_level_count(self.mega_texture.mipmap_levels() as u32);

                    texture
                        .create_view_with_descriptor(&descriptor.into())
                        .map_err(|e| AwsmTextureError::from(AwsmCoreError::texture_view(e)))
                })
                .collect::<Result<Vec<_>>>()?;

            self.gpu_dirty = false;
        }

        Ok(was_gpu_dirty)
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
