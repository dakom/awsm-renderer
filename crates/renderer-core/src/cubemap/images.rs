use crate::command::color::Color;
use crate::cubemap::CubemapImage;
use crate::image::bitmap::create_color;
use crate::image::ImageData;
use crate::texture::mipmap::{calculate_mipmap_levels, generate_mipmaps};
use crate::{
    command::copy_texture::Origin3d,
    error::{AwsmCoreError, Result},
    renderer::AwsmRendererWebGpu,
    texture::{Extent3d, TextureDescriptor, TextureDimension, TextureUsage},
};

pub struct CubemapBitmapColors {
    pub z_positive: Color,
    pub z_negative: Color,
    pub x_positive: Color,
    pub x_negative: Color,
    pub y_positive: Color,
    pub y_negative: Color,
}

pub async fn new_colors(
    colors: CubemapBitmapColors,
    width: u32,
    height: u32,
) -> Result<CubemapImage> {
    let z_positive = create_color(colors.z_positive, width, height, None).await?;
    let z_negative = create_color(colors.z_negative, width, height, None).await?;
    let x_positive = create_color(colors.x_positive, width, height, None).await?;
    let x_negative = create_color(colors.x_negative, width, height, None).await?;
    let y_positive = create_color(colors.y_positive, width, height, None).await?;
    let y_negative = create_color(colors.y_negative, width, height, None).await?;

    Ok(CubemapImage::Images {
        z_positive: ImageData::Bitmap {
            image: z_positive,
            options: None,
        },

        z_negative: ImageData::Bitmap {
            image: z_negative,
            options: None,
        },

        x_positive: ImageData::Bitmap {
            image: x_positive,
            options: None,
        },

        x_negative: ImageData::Bitmap {
            image: x_negative,
            options: None,
        },

        y_positive: ImageData::Bitmap {
            image: y_positive,
            options: None,
        },

        y_negative: ImageData::Bitmap {
            image: y_negative,
            options: None,
        },

        mipmaps: false,
    })
}

pub async fn create_texture(
    gpu: &AwsmRendererWebGpu,
    z_positive: &ImageData,
    z_negative: &ImageData,
    x_positive: &ImageData,
    x_negative: &ImageData,
    y_positive: &ImageData,
    y_negative: &ImageData,
    generate_mipmap: bool,
) -> Result<web_sys::GpuTexture> {
    // Collect all faces in the correct order (required for cubemaps)
    let faces = [
        &x_positive, // +X
        &x_negative, // -X
        &y_positive, // +Y
        &y_negative, // -Y
        &z_positive, // +Z
        &z_negative, // -Z
    ];

    // Validate all faces have the same size and format
    let (width, height) = faces[0].size();
    let format = faces[0].format();

    for (i, face) in faces.iter().enumerate() {
        let (face_width, face_height) = face.size();
        if face_width != width || face_height != height {
            return Err(AwsmCoreError::Cubemap(format!(
                "Face {} size ({}, {}) doesn't match first face size ({}, {})",
                i, face_width, face_height, width, height
            )));
        }
        if face.format() != format {
            return Err(AwsmCoreError::Cubemap(format!(
                "Face {} format {:?} doesn't match first face format {:?}",
                i,
                face.format(),
                format
            )));
        }
    }

    // Ensure the texture is square (cubemap requirement)
    if width != height {
        return Err(AwsmCoreError::Cubemap(format!(
            "Cubemap faces must be square, got {}x{}",
            width, height
        )));
    }

    // Calculate mipmap levels if needed
    let mut usage = TextureUsage::new()
        .with_texture_binding()
        .with_render_attachment()
        .with_copy_dst();

    if generate_mipmap {
        usage = usage.with_storage_binding();
    }

    let mipmap_levels = if generate_mipmap {
        calculate_mipmap_levels(width, height)
    } else {
        1
    };

    // Create texture descriptor for cubemap
    // depth_or_array_layers is 6 for cubemaps (one per face)
    let descriptor =
        TextureDescriptor::new(format, Extent3d::new(width, Some(height), Some(6)), usage)
            .with_dimension(TextureDimension::N2d)
            .with_mip_level_count(mipmap_levels);

    let texture = gpu.create_texture(&descriptor.into())?;

    // Copy each face to the appropriate layer (mip level 0)
    for (face_index, face) in faces.iter().enumerate() {
        let source = face.source_info(None, None)?;
        let dest = crate::image::CopyExternalImageDestInfo::new(&texture)
            .with_origin(Origin3d::new().with_z(face_index as u32))
            .with_mip_level(0)
            .with_premultiplied_alpha(face.premultiplied_alpha());

        gpu.copy_external_image_to_texture(&source.into(), &dest.into(), &face.extent_3d().into())?;
    }

    // Generate mipmaps for the cubemap if requested
    if generate_mipmap {
        // Generate mipmaps for all 6 faces
        generate_mipmaps(gpu, &texture, width, height, 6, true, mipmap_levels).await?;
    }

    Ok(texture)
}
