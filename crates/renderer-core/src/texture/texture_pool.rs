#[cfg(feature = "serde")]
pub mod report;

use std::sync::LazyLock;
use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

use crate::texture::TextureUsage;
use crate::{
    command::copy_texture::Origin3d,
    error::{AwsmCoreError, Result},
    image::{CopyExternalImageDestInfo, ImageData},
    renderer::AwsmRendererWebGpu,
    texture::{
        mipmap::{calculate_mipmap_levels, generate_mipmaps, MipmapTextureKind},
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureFormatKey,
        TextureViewDescriptor, TextureViewDimension,
    },
};

pub struct TexturePool<ID> {
    arrays: BTreeMap<TexturePoolArrayKey, TexturePoolArray<ID>>,
    id_to_array_key: HashMap<ID, TexturePoolArrayKey>,
}

pub struct TexturePoolArray<ID> {
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
    pub mipmap: bool,
    pub images: Vec<(ID, ImageData, TextureColorInfo)>,
    pub gpu_dirty: bool,
    pub gpu_texture: Option<web_sys::GpuTexture>,
    pub gpu_texture_view: Option<web_sys::GpuTextureView>,
}

pub struct TexturePoolEntryInfo<ID> {
    pub id: ID,
    pub array_index: usize,
    pub layer_index: usize,
    pub color: TextureColorInfo,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TextureColorInfo {
    pub mipmap_kind: MipmapTextureKind,
    pub srgb_encoded: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
struct TexturePoolArrayKey {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormatKey,
}

impl<ID: Eq + Hash + Clone> TexturePool<ID> {
    pub fn new() -> Self {
        Self {
            arrays: BTreeMap::new(),
            id_to_array_key: HashMap::new(),
        }
    }

    pub fn add_image(
        &mut self,
        id: ID,
        image: ImageData,
        format: TextureFormat,
        color: TextureColorInfo,
    ) {
        let (width, height) = image.size();

        let array_key = TexturePoolArrayKey {
            width,
            height,
            format: format.into(),
        };

        self.arrays
            .entry(array_key.clone())
            .or_insert_with(|| TexturePoolArray::new(format, width, height))
            .insert(id.clone(), image, color);

        self.id_to_array_key.insert(id, array_key);
    }

    pub fn array_by_index(&self, index: usize) -> Option<&TexturePoolArray<ID>> {
        self.arrays.values().nth(index)
    }

    pub fn arrays_len(&self) -> usize {
        self.arrays.len()
    }

    pub fn entry(&self, id: ID) -> Option<TexturePoolEntryInfo<ID>> {
        let id_array_key = self.id_to_array_key.get(&id)?;

        for (array_index, (array_key, array)) in self.arrays.iter().enumerate() {
            if id_array_key == array_key {
                for (layer_index, (layer_id, _, color)) in array.images.iter().enumerate() {
                    if *layer_id == id {
                        return Some(TexturePoolEntryInfo {
                            id,
                            array_index,
                            layer_index,
                            color: *color,
                        });
                    }
                }
            }
        }

        None
    }

    pub async fn write_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<bool> {
        let mut any_dirty = false;
        for array in self.arrays.values_mut() {
            any_dirty |= array.gpu_dirty;
            array.write_gpu(gpu).await?;
        }

        Ok(any_dirty)
    }

    pub fn textures(&self) -> impl Iterator<Item = &web_sys::GpuTexture> {
        self.arrays
            .values()
            .filter_map(|array| array.gpu_texture.as_ref())
    }

    pub fn texture_views(&self) -> impl Iterator<Item = &web_sys::GpuTextureView> {
        self.arrays
            .values()
            .filter_map(|array| array.gpu_texture_view.as_ref())
    }
}

impl<ID> TexturePoolArray<ID> {
    pub fn new(format: TextureFormat, width: u32, height: u32) -> Self {
        Self {
            format,
            width,
            height,
            mipmap: true,
            images: Vec::new(),
            gpu_dirty: true,
            gpu_texture: None,
            gpu_texture_view: None,
        }
    }

    // returns the index of the inserted image
    pub fn insert(&mut self, id: ID, image: ImageData, color: TextureColorInfo) {
        self.images.push((id, image, color));
        self.gpu_dirty = true;
    }

    pub fn mipmap_levels(&self) -> u32 {
        if self.mipmap {
            calculate_mipmap_levels(self.width, self.height)
        } else {
            1
        }
    }

    pub async fn write_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        if !self.gpu_dirty {
            return Ok(());
        }

        let mipmap_levels = self.mipmap_levels();
        let layers = self.images.len() as u32;

        let texture_usage = if self.mipmap {
            TEXTURE_USAGE_MIPMAP.clone()
        } else {
            TEXTURE_USAGE_NO_MIPMAP.clone()
        };

        let dest_tex = gpu.create_texture(
            &TextureDescriptor::new(
                self.format,
                Extent3d::new(self.width, Some(self.height), Some(layers)),
                texture_usage,
            )
            .with_mip_level_count(mipmap_levels)
            .with_dimension(TextureDimension::N2d)
            .into(),
        )?;

        let mut mipmap_texture_kinds: Vec<MipmapTextureKind> = Vec::new();
        // Write to mip level 0 of each layer
        for (index, (_, image, color)) in self.images.iter().enumerate() {
            let source = image.source_info(None, None)?;

            let dest = CopyExternalImageDestInfo::new(&dest_tex)
                .with_origin(Origin3d::new().with_z(index as u32))
                .with_mip_level(0)
                .with_premultiplied_alpha(image.premultiplied_alpha());

            gpu.copy_external_image_to_texture(
                &source.into(),
                &dest.into(),
                &Extent3d::new(self.width, Some(self.height), Some(1)).into(),
            )?;

            mipmap_texture_kinds.push(color.mipmap_kind);
        }

        if self.mipmap {
            generate_mipmaps(gpu, &dest_tex, &mipmap_texture_kinds, mipmap_levels).await?;
        }

        let dest_view = dest_tex
            .create_view_with_descriptor(
                &TextureViewDescriptor::new(Some("Texture Pool Array View"))
                    .with_dimension(TextureViewDimension::N2dArray)
                    .with_array_layer_count(layers)
                    .with_mip_level_count(mipmap_levels) // Only access mip level 0 for writing
                    .into(),
            )
            .map_err(AwsmCoreError::create_texture_view)?;

        self.gpu_dirty = false;
        self.gpu_texture = Some(dest_tex);
        self.gpu_texture_view = Some(dest_view);

        Ok(())
    }
}

#[cfg(feature = "texture-export")]
static TEXTURE_USAGE_MIPMAP: LazyLock<TextureUsage> = LazyLock::new(|| {
    TextureUsage::new()
        .with_storage_binding()
        .with_texture_binding()
        .with_render_attachment()
        .with_copy_src()
        .with_copy_dst()
});

#[cfg(not(feature = "texture-export"))]
static TEXTURE_USAGE_MIPMAP: LazyLock<TextureUsage> = LazyLock::new(|| {
    TextureUsage::new()
        .with_storage_binding()
        .with_texture_binding()
});

#[cfg(feature = "texture-export")]
static TEXTURE_USAGE_NO_MIPMAP: LazyLock<TextureUsage> =
    LazyLock::new(|| TextureUsage::new().with_storage_binding().with_copy_src());

#[cfg(not(feature = "texture-export"))]
static TEXTURE_USAGE_NO_MIPMAP: LazyLock<TextureUsage> =
    LazyLock::new(|| TextureUsage::new().with_storage_binding());
