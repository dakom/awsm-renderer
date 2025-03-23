use web_sys::HtmlImageElement;
// TODO - use reqwest, don't need awsm_web here
use awsm_web::loaders::image::load as fetch_image;

#[cfg(feature = "exr")]
pub mod exr;

pub enum ImageLoader {
    #[cfg(feature = "exr")]
    Exr(exr::ExrImage),
    HtmlImage(HtmlImageElement)
}

impl ImageLoader {
    cfg_if::cfg_if! {
        if #[cfg(feature = "exr")] {
            pub async fn load_url(url:&str) -> anyhow::Result<Self> {
                if url.contains(".exr") {
                    let exr_image = exr::ExrImage::load_url(url).await?;
                    Ok(Self::Exr(exr_image))
                } else {
                    let image = fetch_image(url.to_string()).await?;
                    Ok(Self::HtmlImage(image))
                }
            }

            pub fn size(&self) -> (usize, usize) {
                match self {
                    Self::Exr(exr) => (exr.width, exr.height),
                    Self::HtmlImage(img) => (img.width() as usize, img.height() as usize)
                }
            }
        } else {
            pub async fn load_url(url:&str) -> Result<Self> {
                let image = fetch_image(url.to_string()).await?;
                Ok(Self::HtmlImage(image))
            }

            pub fn size(&self) -> (usize, usize) {
                match self {
                    Self::HtmlImage(img) => (img.width() as usize, img.height() as usize)
                }
            }

        }
    }

    // TODO - upload to gpu somehow?
}