cfg_if::cfg_if! {
    if #[cfg(feature = "ktx")] {
        pub mod ktx;
        use std::sync::Arc;
    }
}

pub mod images;

use crate::{
    cubemap::images::{CubemapBitmapColors, CubemapSkyGradient},
    error::{AwsmCoreError, Result},
    image::ImageData,
    renderer::AwsmRendererWebGpu,
    texture::{TextureViewDescriptor, TextureViewDimension},
};

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum CubemapImage {
    #[cfg(feature = "ktx")]
    Ktx(Arc<ktx2::Reader<Vec<u8>>>),
    Images {
        z_positive: ImageData,
        z_negative: ImageData,
        x_positive: ImageData,
        x_negative: ImageData,
        y_positive: ImageData,
        y_negative: ImageData,
        mipmaps: bool,
    },
}

impl CubemapImage {
    cfg_if::cfg_if! {
        if #[cfg(feature = "ktx")] {
            pub async fn load_url_ktx(url:&str) -> anyhow::Result<Self> {
                let reader = ktx::load_url(url).await?;

                Ok(CubemapImage::Ktx(Arc::new(reader)))
            }

            // returns mip count as well
            pub async fn create_texture_and_view(
                &self,
                gpu: &AwsmRendererWebGpu,
                label: Option<&str>,
            ) -> Result<(web_sys::GpuTexture, web_sys::GpuTextureView, u32)> {
                let (texture, mip_count) = match self {
                    CubemapImage::Ktx(reader) => {
                        ktx::create_texture(reader, gpu).await
                    },
                    CubemapImage::Images { z_positive, z_negative, x_positive, x_negative, y_positive, y_negative, mipmaps } => {
                        images::create_texture(gpu, z_positive, z_negative, x_positive, x_negative, y_positive, y_negative, *mipmaps).await
                    }
                }?;


                let view = create_texture_view(&texture, label)?;

                Ok((texture, view, mip_count))

            }

            pub async fn new_colors(colors: CubemapBitmapColors, width: u32, height: u32) -> Result<Self> {
                images::new_colors(colors, width, height).await
            }

            pub async fn new_sky_gradient(colors: CubemapSkyGradient, width: u32, height: u32) -> Result<Self> {
                images::new_sky_gradient(colors, width, height).await
            }
        } else {
            pub async fn create_texture_and_view(
                &self,
                gpu: &AwsmRendererWebGpu,
                label: Option<&str>,
            ) -> Result<(web_sys::GpuTexture, web_sys::GpuTextureView, u32)> {
                let (texture, mip_count) = match self {
                    CubemapImage::Images { z_positive, z_negative, x_positive, x_negative, y_positive, y_negative, mipmaps } => {
                        images::create_texture(gpu, z_positive, z_negative, x_positive, x_negative, y_positive, y_negative, *mipmaps).await
                    }
                }?;

                let view = create_texture_view(&texture, label)?;
                Ok((texture, view, mip_count))

            }

            pub async fn new_colors(colors: CubemapBitmapColors, width: u32, height: u32) -> Result<Self> {
                images::new_colors(colors, width, height).await
            }

            pub async fn new_sky_gradient(colors: CubemapSkyGradient, width: u32, height: u32) -> Result<Self> {
                images::new_sky_gradient(colors, width, height).await
            }
        }
    }
}

fn create_texture_view(
    texture: &web_sys::GpuTexture,
    label: Option<&str>,
) -> Result<web_sys::GpuTextureView> {
    texture
        .create_view_with_descriptor(
            &TextureViewDescriptor::new(label)
                .with_dimension(TextureViewDimension::Cube)
                .into(),
        )
        .map_err(AwsmCoreError::create_texture_view)
}
