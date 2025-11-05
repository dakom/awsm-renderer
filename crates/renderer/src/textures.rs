use std::collections::{BTreeSet, HashMap, HashSet};

use awsm_renderer_core::{
    compare::CompareFunction,
    cubemap::CubemapImage,
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
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use thiserror::Error;
use web_sys::{GpuMipmapFilterMode, GpuSupportedLimits};

use crate::{
    bind_groups::{BindGroupCreate, BindGroups},
    error::AwsmError,
    render_passes::{
        material::opaque::render_pass::MaterialOpaqueRenderPass, RenderPassInitContext,
    },
    AwsmRenderer, AwsmRendererLogging,
};

impl AwsmRenderer {
    // this should ideally only be called after all the textures have been loaded
    pub async fn finalize_gpu_textures(&mut self) -> std::result::Result<(), AwsmError> {
        let was_dirty = self
            .textures
            .write_gpu_megatexture(&self.logging, &self.gpu)
            .await?;

        if was_dirty {
            // If the mega texture was changed on the GPU, we need to recreate any render passes
            // that depend on it, as well as any pipelines that depend on those render passes
            let mut render_pass_ctx = RenderPassInitContext {
                gpu: &mut self.gpu,
                pipelines: &mut self.pipelines,
                shaders: &mut self.shaders,
                textures: &mut self.textures,
                render_texture_formats: &mut self.render_textures.formats,
                bind_group_layouts: &mut self.bind_group_layouts,
                pipeline_layouts: &mut self.pipeline_layouts,
            };

            self.bind_groups.mark_create(BindGroupCreate::MegaTexture);

            // Update all the things that depend on opaque materials changing due to textures

            // First, that's the render pass itself - necessary because the actual number of bindings
            // may have changed due to new mega texture arrays being created and this affects the bind group layout
            // and thus the pipeline layout as well, requiring a full recreation of the render pass
            // however, internally, it will clone the bind groups and layouts that aren't affected
            self.render_passes
                .material_opaque
                .mega_texture_changed(&mut render_pass_ctx)
                .await?;

            self.textures
                .mega_texture
                .info(&self.gpu.device.limits())
                .into_report()
                .console_log();
        }

        // Either way, gotta also deal with all the meshes that need their shader/pipelines (re)created
        // because the mega texture change may have affected the dynamically generated number of bindings etc.
        // This isn't so bad, it's okay if it's the same container as before, actual heavy creation uses cache
        let mut has_seen_buffer_info = SecondaryMap::new();
        let mut has_seen_material = SecondaryMap::new();
        for (key, mesh) in self.meshes.iter() {
            if has_seen_buffer_info
                .insert(mesh.buffer_info_key, ())
                .is_none()
                || has_seen_material.insert(mesh.material_key, ()).is_none()
            {
                self.render_passes
                    .material_opaque
                    .pipelines
                    .set_compute_pipeline_key(
                        mesh.buffer_info_key,
                        mesh.material_key,
                        &self.gpu,
                        &mut self.shaders,
                        &mut self.pipelines,
                        &self.render_passes.material_opaque.bind_groups,
                        &self.pipeline_layouts,
                        &self.meshes.buffer_infos,
                        &self.anti_aliasing,
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

pub struct Textures {
    pub gpu_texture_arrays: Vec<web_sys::GpuTexture>,
    pub gpu_texture_array_views: Vec<web_sys::GpuTextureView>,
    pub mega_texture: MegaTexture<TextureKey>,
    pub mega_texture_sampler_set: BTreeSet<SamplerKey>,
    textures: SlotMap<TextureKey, MegaTextureEntryInfo<TextureKey>>,
    cubemaps: SlotMap<CubemapTextureKey, web_sys::GpuTexture>,
    samplers: SlotMap<SamplerKey, web_sys::GpuSampler>,
    sampler_cache: HashMap<SamplerCacheKey, SamplerKey>,
    // We keep a mirror of the sampler address modes so that materials can adjust UVs manually when
    // sampling inside the mega texture. This is especially important for clamp/mirror behaviour.
    sampler_address_modes: SecondaryMap<SamplerKey, (Option<AddressMode>, Option<AddressMode>)>,
    texture_samplers: SecondaryMap<TextureKey, SamplerKey>,
    gpu_megatexture_dirty: bool,
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

impl SamplerCacheKey {
    pub fn allowed_ansiotropy(&self) -> bool {
        match (self.min_filter, self.mag_filter, self.mipmap_filter) {
            (Some(FilterMode::Nearest), _, _)
            | (_, Some(FilterMode::Nearest), _)
            | (_, _, Some(MipmapFilterMode::Nearest)) => false,
            _ => true,
        }
    }
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
            mega_texture: MegaTexture::new(&gpu.device.limits(), 8), // 8 pixels recommended for 16× AF safety
            mega_texture_sampler_set: BTreeSet::new(),
            textures: SlotMap::with_key(),
            samplers: SlotMap::with_key(),
            cubemaps: SlotMap::with_key(),
            sampler_cache: HashMap::new(),
            sampler_address_modes: SecondaryMap::new(),
            texture_samplers: SecondaryMap::new(),
            gpu_megatexture_dirty: false,
        }
    }

    pub fn add_image(
        &mut self,
        image_data: ImageData,
        is_srgb_encoded: bool,
        sampler_key: SamplerKey,
    ) -> Result<TextureKey> {
        let key = self.textures.try_insert_with_key(|key| {
            self.mega_texture
                .add_entries(vec![(image_data, key, is_srgb_encoded, Default::default())])
                .map_err(AwsmTextureError::from)
                .and_then(|mut entries| entries.pop().ok_or(AwsmTextureError::MegaTexture))
        })?;

        self.texture_samplers.insert(key, sampler_key);
        self.mega_texture_sampler_set.insert(sampler_key);

        self.gpu_megatexture_dirty = true;

        Ok(key)
    }

    pub fn insert_cubemap(&mut self, texture: web_sys::GpuTexture) -> CubemapTextureKey {
        self.cubemaps.insert(texture)
    }

    pub fn get_cubemap(&self, key: CubemapTextureKey) -> Result<&web_sys::GpuTexture> {
        self.cubemaps
            .get(key)
            .ok_or(AwsmTextureError::CubemapTextureNotFound(key))
    }

    async fn write_gpu_megatexture(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
    ) -> Result<bool> {
        let was_gpu_dirty = self.gpu_megatexture_dirty;
        if self.gpu_megatexture_dirty {
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

            self.gpu_megatexture_dirty = false;
        }

        Ok(was_gpu_dirty)
    }

    pub fn get_texture_sampler_key(&self, texture_key: TextureKey) -> Result<SamplerKey> {
        self.texture_samplers
            .get(texture_key)
            .copied()
            .ok_or(AwsmTextureError::SamplerForTextureNotFound(texture_key))
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
        let address_mode_u = cache_key.address_mode_u;
        let address_mode_v = cache_key.address_mode_v;
        self.sampler_cache.insert(cache_key, key);
        // Persist the original (U,V) wrap modes so that shader-side helpers can reproduce the
        // desired behaviour after UVs are remapped into the mega texture atlas.
        self.sampler_address_modes
            .insert(key, (address_mode_u, address_mode_v));

        Ok(key)
    }

    pub fn get_sampler(&self, key: SamplerKey) -> Result<&web_sys::GpuSampler> {
        self.samplers
            .get(key)
            .ok_or(AwsmTextureError::SamplerNotFound(key))
    }

    pub fn sampler_address_modes(
        &self,
        key: SamplerKey,
    ) -> (Option<AddressMode>, Option<AddressMode>) {
        self.sampler_address_modes
            .get(key)
            .copied()
            .unwrap_or((None, None))
    }
}

new_key_type! {
    pub struct TextureKey;
}

new_key_type! {
    pub struct SamplerKey;
}

new_key_type! {
    pub struct CubemapTextureKey;
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

    #[error("[texture] sampler for texture not found: {0:?}")]
    SamplerForTextureNotFound(TextureKey),

    #[error("[texture] subemap texture not found: {0:?}")]
    CubemapTextureNotFound(CubemapTextureKey),
}
