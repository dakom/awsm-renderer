use std::sync::LazyLock;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, ImageBitmap};

use crate::error::{AwsmCoreError, Result};

use super::ImageBitmapOptions;

thread_local! {
    static WINDOW: LazyLock<web_sys::Window> = LazyLock::new(|| web_sys::window().unwrap_throw());
}

// let options = web_sys::ImageBitmapOptions::new();
// options.set_premultiply_alpha(web_sys::PremultiplyAlpha::None);
pub async fn load(
    url: String,
    options: Option<ImageBitmapOptions>,
) -> Result<web_sys::ImageBitmap> {
    let resp: web_sys::Response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| AwsmCoreError::Fetch(e.to_string()))?
        .into();
    let js_value = JsFuture::from(resp.blob().map_err(AwsmCoreError::fetch)?)
        .await
        .map_err(AwsmCoreError::fetch)?;
    let blob: Blob = js_value.unchecked_into();

    load_blob(&blob, options).await
}

pub async fn load_blob(
    blob: &Blob,
    options: Option<ImageBitmapOptions>,
) -> Result<web_sys::ImageBitmap> {
    let promise = WINDOW
        .with(|window| match options {
            Some(options) => {
                window.create_image_bitmap_with_blob_and_image_bitmap_options(blob, &options.into())
            }
            None => window.create_image_bitmap_with_blob(blob),
        })
        .map_err(AwsmCoreError::create_image_bitmap)?;
    let js_value = JsFuture::from(promise)
        .await
        .map_err(AwsmCoreError::create_image_bitmap)?;
    Ok(js_value.unchecked_into())
}

pub async fn load_js_value(
    data: &JsValue,
    mime_type: &str,
    options: Option<ImageBitmapOptions>,
) -> Result<ImageBitmap> {
    let blob_opts = BlobPropertyBag::new();
    blob_opts.set_type(mime_type);

    let blob = Blob::new_with_buffer_source_sequence_and_options(
        &js_sys::Array::of1(data).into(),
        &blob_opts,
    )
    .map_err(AwsmCoreError::url_parse)?;

    load_blob(&blob, options).await
}

pub async fn load_u8<T: AsRef<[u8]>>(
    data: T,
    mime_type: &str,
    options: Option<ImageBitmapOptions>,
) -> Result<ImageBitmap> {
    load_js_value(
        // should be fine, load_js_value is just getting a blob with a new url string
        unsafe { &js_sys::Uint8Array::view(data.as_ref()).into() },
        mime_type,
        options,
    )
    .await
}

pub async fn load_u16<T: AsRef<[u16]>>(
    data: T,
    mime_type: &str,
    options: Option<ImageBitmapOptions>,
) -> Result<ImageBitmap> {
    load_js_value(
        // should be fine, load_js_value is just getting a blob with a new url string
        unsafe { &js_sys::Uint16Array::view(data.as_ref()).into() },
        mime_type,
        options,
    )
    .await
}

pub async fn load_u32<T: AsRef<[u32]>>(
    data: T,
    mime_type: &str,
    options: Option<ImageBitmapOptions>,
) -> Result<ImageBitmap> {
    load_js_value(
        // should be fine, load_js_value is just getting a blob with a new url string
        unsafe { &js_sys::Uint32Array::view(data.as_ref()).into() },
        mime_type,
        options,
    )
    .await
}

pub async fn load_i8<T: AsRef<[i8]>>(
    data: T,
    mime_type: &str,
    options: Option<ImageBitmapOptions>,
) -> Result<ImageBitmap> {
    load_js_value(
        // should be fine, load_js_value is just getting a blob with a new url string
        unsafe { &js_sys::Int8Array::view(data.as_ref()).into() },
        mime_type,
        options,
    )
    .await
}

pub async fn load_i16<T: AsRef<[i16]>>(
    data: T,
    mime_type: &str,
    options: Option<ImageBitmapOptions>,
) -> Result<ImageBitmap> {
    load_js_value(
        // should be fine, load_js_value is just getting a blob with a new url string
        unsafe { &js_sys::Int16Array::view(data.as_ref()).into() },
        mime_type,
        options,
    )
    .await
}

pub async fn load_i32<T: AsRef<[i32]>>(
    data: T,
    mime_type: &str,
    options: Option<ImageBitmapOptions>,
) -> Result<ImageBitmap> {
    load_js_value(
        // should be fine, load_js_value is just getting a blob with a new url string
        unsafe { &js_sys::Int32Array::view(data.as_ref()).into() },
        mime_type,
        options,
    )
    .await
}

pub async fn load_f32<T: AsRef<[f32]>>(
    data: T,
    mime_type: &str,
    options: Option<ImageBitmapOptions>,
) -> Result<ImageBitmap> {
    load_js_value(
        // should be fine, load_js_value is just getting a blob with a new url string
        unsafe { &js_sys::Float32Array::view(data.as_ref()).into() },
        mime_type,
        options,
    )
    .await
}

pub async fn load_f64<T: AsRef<[f64]>>(
    data: T,
    mime_type: &str,
    options: Option<ImageBitmapOptions>,
) -> Result<ImageBitmap> {
    load_js_value(
        // should be fine, load_js_value is just getting a blob with a new url string
        unsafe { &js_sys::Float64Array::view(data.as_ref()).into() },
        mime_type,
        options,
    )
    .await
}

fn _same_origin(url: &str) -> Result<bool> {
    if url.starts_with("http://") || url.starts_with("https://") {
        let location_origin = WINDOW
            .with(|window| window.location().origin())
            .map_err(AwsmCoreError::location_origin)?;
        let url_origin = web_sys::Url::new(url)
            .map_err(AwsmCoreError::url_parse)?
            .origin();
        Ok(url_origin == location_origin)
    } else {
        Ok(true)
    }
}
