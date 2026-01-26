//! Image-based lighting (IBL) helpers.

use std::sync::LazyLock;

use awsm_renderer_core::cubemap::CubemapImage;
use awsm_renderer_core::sampler::{AddressMode, FilterMode, MipmapFilterMode};
use awsm_renderer_core::{cubemap::images::CubemapBitmapColors, renderer::AwsmRendererWebGpu};

use crate::error::Result;
use crate::textures::{CubemapTextureKey, SamplerCacheKey, Textures};

/// Image-based lighting textures.
#[derive(Clone)]
pub struct Ibl {
    pub prefiltered_env: IblTexture,
    pub irradiance: IblTexture,
}

impl Ibl {
    /// Creates IBL data from prefiltered and irradiance textures.
    pub fn new(prefiltered_env: IblTexture, irradiance: IblTexture) -> Self {
        Self {
            prefiltered_env,
            irradiance,
        }
    }
}

/// Single IBL cubemap texture and sampler.
#[derive(Clone)]
pub struct IblTexture {
    pub texture_key: CubemapTextureKey,
    pub texture_view: web_sys::GpuTextureView,
    pub sampler: web_sys::GpuSampler,
    pub mip_count: u32,
}

static SAMPLER_CACHE_KEY: LazyLock<SamplerCacheKey> = LazyLock::new(|| SamplerCacheKey {
    address_mode_u: Some(AddressMode::ClampToEdge),
    address_mode_v: Some(AddressMode::ClampToEdge),
    address_mode_w: Some(AddressMode::ClampToEdge),
    mag_filter: Some(FilterMode::Linear),
    min_filter: Some(FilterMode::Linear),
    mipmap_filter: Some(MipmapFilterMode::Linear),
    max_anisotropy: Some(16),
    ..Default::default()
});

impl IblTexture {
    /// Returns the sampler cache key used for IBL textures.
    pub fn sampler_cache_key() -> SamplerCacheKey {
        SAMPLER_CACHE_KEY.clone()
    }

    /// Creates an IBL texture wrapper.
    pub fn new(
        texture_key: CubemapTextureKey,
        texture_view: web_sys::GpuTextureView,
        sampler: web_sys::GpuSampler,
        mip_count: u32,
    ) -> Self {
        Self {
            texture_key,
            texture_view,
            sampler,
            mip_count,
        }
    }

    /// Creates an IBL texture from solid colors.
    pub async fn new_colors(
        gpu: &AwsmRendererWebGpu,
        textures: &mut Textures,
        default_colors: CubemapBitmapColors,
    ) -> Result<Self> {
        let (texture, view, mip_count) = CubemapImage::new_colors(default_colors, 256, 256)
            .await?
            .create_texture_and_view(gpu, Some("IBL Cubemap"))
            .await?;

        let texture_key = textures.insert_cubemap(texture);

        let sampler_key = textures.get_sampler_key(gpu, Self::sampler_cache_key())?;

        let sampler = textures.get_sampler(sampler_key)?.clone();

        Ok(Self::new(texture_key, view, sampler, mip_count))
    }
}
