use awsm_renderer::{
    core::cubemap::{images::CubemapBitmapColors, CubemapImage},
    lights::ibl::{Ibl, IblTexture},
    AwsmRenderer,
};

use crate::prelude::*;

pub struct IblCubemaps {
    pub prefiltered_env: CubemapImage,
    pub irradiance: CubemapImage,
}

pub async fn load_from_path(path: &str) -> Result<IblCubemaps> {
    async fn load_ibl_image(filename: &str, environment_path: &str) -> Result<CubemapImage> {
        let filename = if CONFIG.cache_buster {
            format!("{filename}?cb={}", js_sys::Date::now())
        } else {
            filename.to_string()
        };

        CubemapImage::load_url_ktx(&format!(
            "{}/{}/{}",
            CONFIG.environment_url, environment_path, filename
        ))
        .await
    }

    Ok(IblCubemaps {
        prefiltered_env: load_ibl_image("env.ktx2", path).await?,
        irradiance: load_ibl_image("irradiance.ktx2", path).await?,
    })
}

pub async fn load_from_colors(colors: CubemapBitmapColors) -> Result<IblCubemaps> {
    Ok(IblCubemaps {
        prefiltered_env: CubemapImage::new_colors(colors.clone(), 1024, 1024).await?,
        irradiance: CubemapImage::new_colors(colors, 32, 32).await?,
    })
}
