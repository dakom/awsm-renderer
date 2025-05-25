use crate::command::copy_texture::Origin3d;
use crate::error::Result;
use crate::renderer::AwsmRendererWebGpu;
use crate::texture::{Extent3d, TextureAspect, TextureDescriptor, TextureFormat, TextureUsage};
use std::borrow::Cow;
use mipmap::generate_mipmaps;
use wasm_bindgen::prelude::*;

pub mod mipmap;
pub mod bitmap;
#[cfg(feature = "exr")]
pub mod exr;

#[derive(Clone, Debug)]
pub enum ImageData {
    #[cfg(feature = "exr")]
    Exr(Box<exr::ExrImage>),
    Bitmap {
        image: web_sys::ImageBitmap,
        options: Option<ImageBitmapOptions>,
    }
}

impl ImageData {
    cfg_if::cfg_if! {
        if #[cfg(feature = "exr")] {
            pub async fn load_url(url:&str, options: Option<ImageBitmapOptions>) -> anyhow::Result<Self> {
                if url.contains(".exr") {
                    let exr_image = exr::ExrImage::load_url(url).await?;
                    Ok(Self::Exr(Box::new(exr_image)))
                } else {
                    let image = bitmap::load(url.to_string(), options.clone()).await?;
                    Ok(Self::Bitmap{image, options})
                }
            }
        } else {
            pub async fn load_url(url:&str, options: Option<ImageBitmapOptions>) -> Result<Self> {
                let image = bitmap::load(url.to_string(), options.clone()).await?;
                Ok(Self::Bitmap{image, options})
            }
        }
    }

    pub fn format(&self) -> TextureFormat {
        match self {
            // TODO - is this right?
            #[cfg(feature = "exr")]
            Self::Exr(_) => TextureFormat::Rgba32float,

            Self::Bitmap{..} => TextureFormat::Rgba8unorm,
        }
    }

    pub fn premultiplied_alpha(&self) -> bool {
        match self {
            // TODO - is this right?
            #[cfg(feature = "exr")]
            Self::Exr(_) => true, // EXR images are typically premultiplied

            Self::Bitmap{options, ..} => options
                .as_ref()
                .map(|opts| matches!(opts.premultiply_alpha, Some(PremultiplyAlpha::Premultiply)))
                .unwrap_or(false),
        }
    }

    pub fn size(&self) -> Extent3d {
        match self {
            #[cfg(feature = "exr")]
            Self::Exr(exr) => Extent3d {
                width: exr.width as u32,
                height: Some(exr.height as u32),
                depth_or_array_layers: None,
            },

            Self::Bitmap{image, ..} => Extent3d {
                width: image.width(),
                height: Some(image.height()),
                depth_or_array_layers: None,
            },
        }
    }

    pub fn js_obj(&self) -> Result<Cow<'_, js_sys::Object>> {
        match self {
            #[cfg(feature = "exr")]
            Self::Exr(exr) => exr.js_obj(),

            Self::Bitmap{image, ..} => {
                let js_value = image.unchecked_ref();
                Ok(Cow::Borrowed(js_value))
            }
        }
    }

    pub fn source_info(
        &self,
        origin: Option<[f32; 2]>,
        flip_y: Option<bool>,
    ) -> Result<CopyExternalImageSourceInfo> {
        Ok(CopyExternalImageSourceInfo {
            flip_y,
            origin,
            source: self.js_obj()?,
        })
    }

    pub async fn create_texture<'a>(
        &self,
        gpu: &AwsmRendererWebGpu,
        source_info: Option<CopyExternalImageSourceInfo<'a>>,
        generate_mipmap: bool,
    ) -> Result<web_sys::GpuTexture> {
        let mut usage = TextureUsage::new()
            .with_texture_binding()
            // needed because `copy_external_image_to_texture` renders to the texture internally, part of browser WebGPU implementation
            .with_render_attachment()
            .with_copy_dst();

        if generate_mipmap {
            usage = usage.with_storage_binding();
        }

        let source = match source_info {
            Some(info) => info,
            None => CopyExternalImageSourceInfo {
                flip_y: None,
                origin: None,
                source: self.js_obj()?,
            },
        };

        let mut descriptor = TextureDescriptor::new(self.format(), self.size(), usage);
        let mipmap_levels = if generate_mipmap {
            let mipmap_levels = mipmap::calculate_mipmap_levels(
                self.size().width,
                self.size().height.unwrap_or(1),
            );

            descriptor = descriptor
                .with_mip_level_count(mipmap_levels);

            Some(mipmap_levels)
        } else {
            None
        };

        let texture = gpu.create_texture(&descriptor.into())?;

        let mut dest = CopyExternalImageDestInfo::new(&texture)
            .with_premultiplied_alpha(self.premultiplied_alpha());

        if generate_mipmap {
            dest = dest.with_mip_level(0);
        }
        gpu.copy_external_image_to_texture(&source.into(), &dest.into(), &self.size().into())?;

        if let Some(mipmap_levels) = mipmap_levels {
            generate_mipmaps(gpu, &texture, self.size().width, self.size().height.unwrap_or(1), mipmap_levels).await?;
        }

        Ok(texture)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImageBitmapOptions {
    // https://docs.rs/web-sys/latest/web_sys/struct.ImageBitmapOptions.html
    pub color_space_conversion: Option<ColorSpaceConversion>,
    pub image_orientation: Option<ImageOrientation>,
    pub premultiply_alpha: Option<PremultiplyAlpha>,
    pub resize_height: Option<u32>,
    pub resize_width: Option<u32>,
    pub resize_quality: Option<ResizeQuality>,
}

impl ImageBitmapOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_color_space_conversion(
        mut self,
        color_space_conversion: ColorSpaceConversion,
    ) -> Self {
        self.color_space_conversion = Some(color_space_conversion);
        self
    }

    pub fn with_image_orientation(mut self, image_orientation: ImageOrientation) -> Self {
        self.image_orientation = Some(image_orientation);
        self
    }

    pub fn with_premultiply_alpha(mut self, premultiply_alpha: PremultiplyAlpha) -> Self {
        self.premultiply_alpha = Some(premultiply_alpha);
        self
    }

    pub fn with_resize_height(mut self, resize_height: u32) -> Self {
        self.resize_height = Some(resize_height);
        self
    }

    pub fn with_resize_width(mut self, resize_width: u32) -> Self {
        self.resize_width = Some(resize_width);
        self
    }

    pub fn with_resize_quality(mut self, resize_quality: ResizeQuality) -> Self {
        self.resize_quality = Some(resize_quality);
        self
    }
}

// https://docs.rs/web-sys/latest/web_sys/enum.ColorSpaceConversion.html
pub type ColorSpaceConversion = web_sys::ColorSpaceConversion;
// https://docs.rs/web-sys/latest/web_sys/enum.ImageOrientation.html
pub type ImageOrientation = web_sys::ImageOrientation;
// https://docs.rs/web-sys/latest/web_sys/enum.PremultiplyAlpha.html
pub type PremultiplyAlpha = web_sys::PremultiplyAlpha;
// https://docs.rs/web-sys/latest/web_sys/enum.ResizeQuality.html
pub type ResizeQuality = web_sys::ResizeQuality;

// Can create this from ImageData.source_info()
pub struct CopyExternalImageSourceInfo<'a> {
    pub flip_y: Option<bool>,
    pub origin: Option<[f32; 2]>,
    pub source: Cow<'a, js_sys::Object>,
}

impl<'a> CopyExternalImageSourceInfo<'a> {
    pub fn new(source: Cow<'a, js_sys::Object>) -> Self {
        Self {
            flip_y: None,
            origin: None,
            source,
        }
    }
}

pub struct CopyExternalImageDestInfo<'a> {
    pub texture: &'a web_sys::GpuTexture,
    pub aspect: Option<TextureAspect>,
    pub mip_level: Option<u32>,
    pub origin: Option<Origin3d>,
    pub premultiplied_alpha: Option<bool>,
}

impl<'a> CopyExternalImageDestInfo<'a> {
    pub fn new(texture: &'a web_sys::GpuTexture) -> Self {
        Self {
            aspect: None,
            mip_level: None,
            origin: None,
            premultiplied_alpha: None,
            texture,
        }
    }

    pub fn with_aspect(mut self, aspect: TextureAspect) -> Self {
        self.aspect = Some(aspect);
        self
    }
    pub fn with_mip_level(mut self, mip_level: u32) -> Self {
        self.mip_level = Some(mip_level);
        self
    }
    pub fn with_origin(mut self, origin: Origin3d) -> Self {
        self.origin = Some(origin);
        self
    }
    pub fn with_premultiplied_alpha(mut self, premultiplied_alpha: bool) -> Self {
        self.premultiplied_alpha = Some(premultiplied_alpha);
        self
    }
}


impl From<CopyExternalImageSourceInfo<'_>> for web_sys::GpuCopyExternalImageSourceInfo {
    fn from(info: CopyExternalImageSourceInfo) -> Self {
        // https://developer.mozilla.org/en-US/docs/Web/API/GPUQueue/copyExternalImageToTexture#source
        // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuCopyExternalImageSourceInfo.html
        let info_js = web_sys::GpuCopyExternalImageSourceInfo::new(&info.source);

        if let Some(flip_y) = info.flip_y {
            info_js.set_flip_y(flip_y);
        }

        if let Some(origin) = info.origin {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(
                obj.as_ref(),
                &JsValue::from("x"),
                &JsValue::from_f64(origin[0] as f64),
            )
            .unwrap();
            js_sys::Reflect::set(
                obj.as_ref(),
                &JsValue::from("y"),
                &JsValue::from_f64(origin[1] as f64),
            )
            .unwrap();
            info_js.set_origin(&obj);
        }

        info_js
    }
}

impl From<CopyExternalImageDestInfo<'_>> for web_sys::GpuCopyExternalImageDestInfo {
    fn from(info: CopyExternalImageDestInfo) -> Self {
        // https://developer.mozilla.org/en-US/docs/Web/API/GPUQueue/copyExternalImageToTexture#destination
        // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuCopyExternalImageDestInfo.html
        let info_js = web_sys::GpuCopyExternalImageDestInfo::new(info.texture);

        if let Some(aspect) = info.aspect {
            info_js.set_aspect(aspect);
        }
        if let Some(mip_level) = info.mip_level {
            info_js.set_mip_level(mip_level);
        }
        if let Some(origin) = info.origin {
            info_js.set_origin(&web_sys::GpuOrigin3dDict::from(origin));
        }
        if let Some(premultiplied_alpha) = info.premultiplied_alpha {
            info_js.set_premultiplied_alpha(premultiplied_alpha);
        }

        info_js
    }
}

impl From<ImageBitmapOptions> for web_sys::ImageBitmapOptions {
    fn from(options: ImageBitmapOptions) -> web_sys::ImageBitmapOptions {
        let js_options = web_sys::ImageBitmapOptions::new();

        if let Some(color_space_conversion) = options.color_space_conversion {
            js_options.set_color_space_conversion(color_space_conversion);
        }

        if let Some(image_orientation) = options.image_orientation {
            js_options.set_image_orientation(image_orientation);
        }

        if let Some(premultiply_alpha) = options.premultiply_alpha {
            js_options.set_premultiply_alpha(premultiply_alpha);
        }

        if let Some(resize_height) = options.resize_height {
            js_options.set_resize_height(resize_height);
        }

        if let Some(resize_width) = options.resize_width {
            js_options.set_resize_width(resize_width);
        }

        if let Some(resize_quality) = options.resize_quality {
            js_options.set_resize_quality(resize_quality);
        }

        js_options
    }
}
