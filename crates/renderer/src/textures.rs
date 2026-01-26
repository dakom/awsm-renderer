use std::{collections::HashMap, sync::LazyLock};

use awsm_renderer_core::{
    buffers::{BufferDescriptor, BufferUsage},
    compare::CompareFunction,
    error::AwsmCoreError,
    image::ImageData,
    renderer::AwsmRendererWebGpu,
    sampler::{AddressMode, FilterMode, MipmapFilterMode, SamplerDescriptor},
    texture::{
        texture_pool::{TextureColorInfo, TexturePool, TexturePoolEntryInfo},
        TextureFormat,
    },
};
use indexmap::IndexSet;
use ordered_float::OrderedFloat;
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::{BindGroupCreate, BindGroups},
    buffer::dynamic_uniform::DynamicUniformBuffer,
    buffer::helpers::write_buffer_with_dirty_ranges,
    error::AwsmError,
    render_passes::RenderPassInitContext,
    AwsmRenderer, AwsmRendererLogging,
};

static TEXTURE_TRANSFORM_BUFFER_USAGE: LazyLock<BufferUsage> =
    LazyLock::new(|| BufferUsage::new().with_storage().with_copy_dst());

pub const TEXTURE_TRANSFORMS_INITIAL_CAPACITY: usize = 32; // 32 elements is a good starting point
pub const TEXTURE_TRANSFORMS_BYTE_SIZE: usize = 32; // 32 bytes per texture transform (must match shader struct size)

impl AwsmRenderer {
    // this should ideally only be called after all the textures have been loaded
    pub async fn finalize_gpu_textures(&mut self) -> std::result::Result<(), AwsmError> {
        let was_dirty = self
            .textures
            .write_gpu_texture_pool(&self.logging, &self.gpu)
            .await?;

        if was_dirty {
            // If the pool was changed on the GPU, we need to recreate any render passes
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

            self.bind_groups.mark_create(BindGroupCreate::TexturePool);

            // Update render passes that depend on texture pool size (affects bind group layouts
            // and pipeline layouts due to dynamically generated texture array/sampler bindings).
            //
            // OPAQUE: Pipelines are based only on global parameters (MSAA, mipmaps, texture pool size),
            // so texture_pool_changed() fully recreates all pipeline variants. No per-mesh iteration needed.
            //
            // TRANSPARENT: Pipelines depend on per-mesh attributes, so texture_pool_changed() only
            // updates bind groups and creates a new pipeline layout. The actual per-mesh pipelines
            // must be recreated separately below using the new layout.
            self.render_passes
                .material_opaque
                .texture_pool_changed(&mut render_pass_ctx)
                .await?;

            self.render_passes
                .material_transparent
                .texture_pool_changed(&mut render_pass_ctx)
                .await?;
        }

        // Recreate transparent pass pipelines for each mesh (and _only_ transparent!)
        // These depend on per-mesh attributes (unlike opaque which uses only global parameters),
        // so we must iterate through meshes to create pipelines with the (potentially new) layout.
        // Caching ensures this is efficient when pipelines already exist.
        let mut has_seen_buffer_info = SecondaryMap::new();
        let mut has_seen_material = SecondaryMap::new();
        for (key, mesh) in self.meshes.iter() {
            let buffer_info_key = self.meshes.buffer_info_key(key)?;
            if has_seen_buffer_info
                .insert(buffer_info_key, ())
                .is_none()
                || has_seen_material.insert(mesh.material_key, ()).is_none()
            {
                self.render_passes
                    .material_transparent
                    .pipelines
                    .set_render_pipeline_key(
                        &self.gpu,
                        mesh,
                        key,
                        buffer_info_key,
                        &mut self.shaders,
                        &mut self.pipelines,
                        &self.render_passes.material_transparent.bind_groups,
                        &self.pipeline_layouts,
                        &self.meshes.buffer_infos,
                        &self.anti_aliasing,
                        &self.textures,
                        &self.render_textures.formats,
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

pub struct Textures {
    pub pool: TexturePool<TextureKey>,
    pub pool_sampler_set: IndexSet<SamplerKey>,
    pub texture_transform_identity_offset: usize,
    pool_textures: SlotMap<TextureKey, TexturePoolEntryInfo<TextureKey>>,
    cubemaps: SlotMap<CubemapTextureKey, web_sys::GpuTexture>,
    samplers: SlotMap<SamplerKey, web_sys::GpuSampler>,
    sampler_cache: HashMap<SamplerCacheKey, SamplerKey>,
    // We keep a mirror of the sampler address modes so that materials can adjust UVs manually when
    sampler_address_modes: SecondaryMap<SamplerKey, (Option<AddressMode>, Option<AddressMode>)>,
    texture_transforms: SlotMap<TextureTransformKey, ()>,
    texture_transforms_buffer: DynamicUniformBuffer<TextureTransformKey>,
    texture_transforms_gpu_dirty: bool,
    pub(crate) texture_transforms_gpu_buffer: web_sys::GpuBuffer,
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

#[derive(Debug, Clone, PartialEq)]
pub struct TextureTransform {
    pub offset: [f32; 2],
    pub origin: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
}

impl TextureTransform {
    pub fn identity() -> Self {
        Self {
            offset: [0.0, 0.0],
            origin: [0.0, 0.0],
            rotation: 0.0,
            scale: [1.0, 1.0],
        }
    }

    pub fn as_gpu_bytes(&self) -> [u8; TEXTURE_TRANSFORMS_BYTE_SIZE] {
        let mut bytes = [0u8; TEXTURE_TRANSFORMS_BYTE_SIZE];

        let sx = self.scale[0];
        let sy = self.scale[1];
        let ox = self.offset[0];
        let oy = self.offset[1];
        let px = self.origin[0];
        let py = self.origin[1];

        let c = self.rotation.cos();
        let s = self.rotation.sin();

        // M = R * S
        // glTF rotation matrix (counter-clockwise, with V pointing down):
        // [ cos   sin ] * [ sx  0  ]   =   [ cos*sx   sin*sy ]
        // [ -sin  cos ]   [ 0   sy ]       [ -sin*sx  cos*sy ]
        let m00 = c * sx;
        let m01 = s * sy;
        let m10 = -s * sx;
        let m11 = c * sy;

        // B = offset + origin - M * origin
        let mx_px = m00 * px + m01 * py;
        let my_py = m10 * px + m11 * py;

        let bx = ox + px - mx_px;
        let by = oy + py - my_py;

        bytes[0..4].copy_from_slice(&m00.to_le_bytes());
        bytes[4..8].copy_from_slice(&m01.to_le_bytes());
        bytes[8..12].copy_from_slice(&m10.to_le_bytes());
        bytes[12..16].copy_from_slice(&m11.to_le_bytes());
        bytes[16..20].copy_from_slice(&bx.to_le_bytes());
        bytes[20..24].copy_from_slice(&by.to_le_bytes());

        bytes
    }
}

impl Textures {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let samplers = SlotMap::with_key();
        let sampler_cache = HashMap::new();
        let sampler_address_modes = SecondaryMap::new();

        let texture_transforms_gpu_buffer = gpu.create_buffer(
            &BufferDescriptor::new(
                Some("Texture Transforms"),
                TEXTURE_TRANSFORMS_INITIAL_CAPACITY * TEXTURE_TRANSFORMS_BYTE_SIZE,
                *TEXTURE_TRANSFORM_BUFFER_USAGE,
            )
            .into(),
        )?;
        let mut texture_transforms_buffer = DynamicUniformBuffer::new(
            TEXTURE_TRANSFORMS_INITIAL_CAPACITY,
            TEXTURE_TRANSFORMS_BYTE_SIZE,
            None,
            Some("Texture Transforms".to_string()),
        );

        let mut texture_transforms = SlotMap::with_key();

        let texture_transform_identity_offset = {
            let transform = TextureTransform::identity();
            let key = texture_transforms.insert(());

            texture_transforms_buffer.update(key, &transform.as_gpu_bytes());

            texture_transforms_buffer
                .offset(key)
                .expect("just inserted key must have offset")
        };

        Ok(Self {
            pool: TexturePool::new(),
            pool_sampler_set: IndexSet::new(),
            pool_textures: SlotMap::with_key(),
            cubemaps: SlotMap::with_key(),
            texture_transforms,
            texture_transforms_buffer,
            texture_transforms_gpu_buffer,
            texture_transforms_gpu_dirty: true,
            texture_transform_identity_offset,
            samplers,
            sampler_cache,
            sampler_address_modes,
        })
    }

    pub fn add_image(
        &mut self,
        image_data: ImageData,
        texture_format: TextureFormat,
        sampler_key: SamplerKey,
        color: TextureColorInfo,
    ) -> Result<TextureKey> {
        let key = self.pool_textures.try_insert_with_key(|key| {
            self.pool.add_image(key, image_data, texture_format, color);
            self.pool
                .entry(key)
                .ok_or(AwsmTextureError::TextureNotFound(key))
        })?;

        self.pool_sampler_set.insert(sampler_key);

        Ok(key)
    }

    pub fn insert_texture_transform(
        &mut self,
        transform: &TextureTransform,
    ) -> TextureTransformKey {
        let key = self.texture_transforms.insert(());
        self.update_texture_transform(key, transform);
        key
    }
    pub fn update_texture_transform(
        &mut self,
        key: TextureTransformKey,
        transform: &TextureTransform,
    ) {
        let bytes = transform.as_gpu_bytes();
        self.texture_transforms_buffer.update(key, &bytes);
        self.texture_transforms_gpu_dirty = true;
    }

    pub fn remove_texture_transform(&mut self, key: TextureTransformKey) {
        self.texture_transforms_buffer.remove(key);
        self.texture_transforms_gpu_dirty = true;
    }

    pub fn get_texture_transform_offset(&self, key: TextureTransformKey) -> Option<usize> {
        self.texture_transforms_buffer.offset(key)
    }

    pub fn get_texture_transform_slot_index(&self, key: TextureTransformKey) -> Option<usize> {
        self.texture_transforms_buffer.slot_index(key)
    }

    pub fn insert_cubemap(&mut self, texture: web_sys::GpuTexture) -> CubemapTextureKey {
        self.cubemaps.insert(texture)
    }

    pub fn get_cubemap(&self, key: CubemapTextureKey) -> Result<&web_sys::GpuTexture> {
        self.cubemaps
            .get(key)
            .ok_or(AwsmTextureError::CubemapTextureNotFound(key))
    }

    async fn write_gpu_texture_pool(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
    ) -> Result<bool> {
        let _maybe_span_guard = if logging.render_timings {
            Some(tracing::span!(tracing::Level::INFO, "Textures GPU write").entered())
        } else {
            None
        };

        self.pool.write_gpu(gpu).await.map_err(|e| e.into())
    }

    pub fn write_texture_transforms_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.texture_transforms_gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Texture Transforms GPU write").entered())
            } else {
                None
            };

            let mut resized = false;
            if let Some(new_size) = self.texture_transforms_buffer.take_gpu_needs_resize() {
                self.texture_transforms_gpu_buffer = gpu.create_buffer(
                    &BufferDescriptor::new(
                        Some("Texture Transforms"),
                        new_size,
                        *TEXTURE_TRANSFORM_BUFFER_USAGE,
                    )
                    .into(),
                )?;

                bind_groups.mark_create(BindGroupCreate::TextureTransformsResize);
                resized = true;
            }

            if resized {
                self.texture_transforms_buffer.clear_dirty_ranges();
                gpu.write_buffer(
                    &self.texture_transforms_gpu_buffer,
                    None,
                    self.texture_transforms_buffer.raw_slice(),
                    None,
                    None,
                )?;
            } else {
                let ranges = self.texture_transforms_buffer.take_dirty_ranges();
                write_buffer_with_dirty_ranges(
                    gpu,
                    &self.texture_transforms_gpu_buffer,
                    self.texture_transforms_buffer.raw_slice(),
                    ranges,
                )?;
            }

            self.texture_transforms_gpu_dirty = false;
        }
        Ok(())
    }

    pub fn get_entry(&self, key: TextureKey) -> Result<&TexturePoolEntryInfo<TextureKey>> {
        self.pool_textures
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

        create_sampler_key(
            gpu,
            cache_key,
            &mut self.samplers,
            &mut self.sampler_cache,
            &mut self.sampler_address_modes,
        )
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

fn create_sampler_key(
    gpu: &AwsmRendererWebGpu,
    cache_key: SamplerCacheKey,
    samplers: &mut SlotMap<SamplerKey, web_sys::GpuSampler>,
    sampler_cache: &mut HashMap<SamplerCacheKey, SamplerKey>,
    sampler_address_modes: &mut SecondaryMap<
        SamplerKey,
        (Option<AddressMode>, Option<AddressMode>),
    >,
) -> Result<SamplerKey> {
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

    // tracing::info!("address_mode_u: {address_mode_u:?}, address_mode_v: {address_mode_v:?}, address_mode_w: {address_mode_w:?}, compare: {compare:?}, lod_min_clamp: {lod_min_clamp:?}, lod_max_clamp: {lod_max_clamp:?}, max_anisotropy: {max_anisotropy:?}, mag_filter: {mag_filter:?}, min_filter: {min_filter:?}, mipmap_filter: {mipmap_filter:?}",
    //     address_mode_u = cache_key.address_mode_u,
    //     address_mode_v = cache_key.address_mode_v,
    //     address_mode_w = cache_key.address_mode_w,
    //     compare = cache_key.compare,
    //     lod_min_clamp = cache_key.lod_min_clamp,
    //     lod_max_clamp = cache_key.lod_max_clamp,
    //     max_anisotropy = cache_key.max_anisotropy,
    //     mag_filter = cache_key.mag_filter,
    //     min_filter = cache_key.min_filter,
    //     mipmap_filter = cache_key.mipmap_filter,
    // );

    let sampler = gpu.create_sampler(Some(&descriptor.into()));

    let key = samplers.insert(sampler);
    let address_mode_u = cache_key.address_mode_u;
    let address_mode_v = cache_key.address_mode_v;
    sampler_cache.insert(cache_key, key);
    // Persist the original (U,V) wrap modes so that shader-side helpers can reproduce the
    sampler_address_modes.insert(key, (address_mode_u, address_mode_v));

    Ok(key)
}

new_key_type! {
    pub struct TextureKey;
}

new_key_type! {
    pub struct TextureTransformKey;
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

    #[error("[texture] pool failure")]
    Pool,

    #[error("[texture] sampler not found: {0:?}")]
    SamplerNotFound(SamplerKey),

    #[error("[texture] texture not found: {0:?}")]
    TextureNotFound(TextureKey),

    #[error("[texture] subemap texture not found: {0:?}")]
    CubemapTextureNotFound(CubemapTextureKey),

    #[error("[texture] no clamp sampler found in mega-texture")]
    NoClampSamplerInMegaTexture,
}
