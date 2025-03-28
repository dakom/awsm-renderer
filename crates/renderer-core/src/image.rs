pub mod element;
#[cfg(feature = "exr")]
pub mod exr;

use web_sys::HtmlImageElement;

#[derive(Clone, Debug)]
pub enum ImageLoader {
    #[cfg(feature = "exr")]
    Exr(Box<exr::ExrImage>),
    HtmlImage(HtmlImageElement),
}

impl ImageLoader {
    cfg_if::cfg_if! {
        if #[cfg(feature = "exr")] {
            pub async fn load_url(url:&str) -> anyhow::Result<Self> {
                if url.contains(".exr") {
                    let exr_image = exr::ExrImage::load_url(url).await?;
                    Ok(Self::Exr(Box::new(exr_image)))
                } else {
                    let image = element::load(url.to_string()).await?;
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
                let image = element::load(url.to_string()).await?;
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
