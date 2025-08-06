/// A "MegaTexture" can be used to store a large number of images
/// in a way that ultimately maps to a collection of large GPU texture arrays.
///
/// It is parameterized over `ID` so that entries can be identified no matter where they are in the atlas.
///
/// It's structured as a collection of atlases, each of which contains multiple layers.
/// In other words, each atlas is a 2D texture array, and each layer is an item in that texture-array
///
/// It grows dynamically as needed, in two ways:
///
/// 1. Keep trying to add images to the current layer until it is full (fullness is max width/height)
/// 2. Keep trying to add layers to the current atlas until it is full (fullness is max depth)
/// 3. Keep trying to add atlases indefinitely (if we run out here, we're out of resources)
///
/// The limits are determined by the GPU's capabilities, such as max texture size, max texture array layers, and max buffer size.
///
/// These limits can be quite high by requesting to raise those limits from the device, which is made easy
/// by initializing the gpu builder with `DeviceRequestLimits::max_all()`
///
/// Going further would require a more complex system, such as a streaming MegaTexture
///
/// Each layer is packed using a 2D bin packing algorithm, which allows for efficient use of space.
///
/// Lastly, each original image is tracked with its 3d index into the MegaTexture (atlas, layer, entry)
/// as well as the pixel offset in the layer texture, UV offset, and UV scale.
pub mod pipeline;
pub mod writer;

use std::collections::HashMap;

use crate::error::{AwsmCoreError, Result};
use binpack2d::{
    maxrects::{Heuristic, MaxRectsBin},
    Dimension,
};

use crate::image::ImageData;

pub struct MegaTexture<ID> {
    // width and height of each layer in each atlas
    pub texture_size: u32,
    // the depth of each atlas, i.e. how many layers it can have
    pub atlas_depth: u32,
    // padding around each image, useful for mipmapping and avoiding artifacts
    pub padding: u32,

    pub mipmap: bool,

    pub(super) atlases: Vec<MegaTextureAtlas<ID>>,
    // This is a lookup table for the index of each image in the mega texture
    lookup: HashMap<ID, MegaTextureIndex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MegaTextureIndex {
    pub atlas_index: usize,
    pub layer_index: usize,
    pub entry_index: usize,
}

#[derive(Clone)]
pub struct MegaTextureSize {
    inner_len: Vec<Vec<usize>>,
    inner_size: Vec<Vec<Vec<(u32, u32)>>>,
    texture_size: u32,
    max_depth: u32,
    max_size: (u32, u32),
    max_size_per_bind_group: (u32, u32),
}

impl std::fmt::Debug for MegaTextureSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MegaTextureSize")
            .field("atlas_len", &self.atlas_len())
            .field("layer_per_atlas_len", &self.layer_per_atlas_len())
            .field(
                "entry_per_layer_per_atlas_len",
                &self.entry_per_layer_per_atlas_len(),
            )
            .field("total_entries_len", &self.total_entries_len())
            .field("total_layers_len", &self.total_layers_len())
            .field("max_atlas_size", &self.max_atlas_size())
            .field("atlas_sizes", &self.atlas_sizes())
            .field("layer_per_atlas_size", &self.layer_per_atlas_size())
            .field(
                "entry_per_layer_per_atlas_size",
                &self.entry_per_layer_per_atlas_size(),
            )
            .field("total_size", &self.total_size())
            .field("max_size", &self.max_size)
            .field("max_size_per_bind_group", &self.max_size_per_bind_group)
            .field("perc_used", &format!("{}%", self.perc_used()))
            .field("perc_free", &format!("{}%", self.perc_free()))
            .finish()
    }
}

impl MegaTextureSize {
    pub fn atlas_len(&self) -> usize {
        self.inner_len.len()
    }

    pub fn layer_per_atlas_len(&self) -> Vec<usize> {
        self.inner_len.iter().map(|l| l.len()).collect()
    }

    pub fn entry_per_layer_per_atlas_len(&self) -> Vec<Vec<usize>> {
        self.inner_len
            .iter()
            .map(|l| l.iter().map(|e| *e).collect())
            .collect()
    }

    pub fn total_entries_len(&self) -> usize {
        self.inner_len
            .iter()
            .map(|l| l.iter().map(|e| *e).sum::<usize>())
            .sum()
    }

    pub fn total_layers_len(&self) -> usize {
        self.inner_len.iter().map(|l| l.len()).sum()
    }

    pub fn max_atlas_size(&self) -> (u32, u32) {
        let max_size = self.max_depth * self.texture_size;
        (max_size, max_size)
    }

    pub fn total_size(&self) -> (u32, u32) {
        let mut width = 0;
        let mut height = 0;
        for (w, h) in self.atlas_sizes() {
            width += w;
            height += h;
        }
        (width, height)
    }

    pub fn atlas_sizes(&self) -> Vec<(u32, u32)> {
        let mut out = Vec::new();

        for layer in &self.inner_size {
            let mut width = 0;
            let mut height = 0;
            for entries in layer {
                for entry in entries {
                    width += entry.0;
                    height += entry.1;
                }
            }
            out.push((width, height));
        }

        out
    }

    pub fn layer_per_atlas_size(&self) -> Vec<Vec<(u32, u32)>> {
        let mut out = Vec::new();

        for layer in &self.inner_size {
            let mut out_l = Vec::new();
            for entries in layer {
                let mut width = 0;
                let mut height = 0;
                for entry in entries {
                    width += entry.0;
                    height += entry.1;
                }
                out_l.push((width, height));
            }
            out.push(out_l);
        }

        out
    }

    pub fn entry_per_layer_per_atlas_size(&self) -> Vec<Vec<Vec<(u32, u32)>>> {
        let mut out = Vec::new();

        for layer in &self.inner_size {
            let mut out_l = Vec::new();
            for entries in layer {
                let mut out_e = Vec::new();
                for entry in entries {
                    out_e.push(*entry);
                }
                out_l.push(out_e);
            }
            out.push(out_l);
        }

        out
    }

    pub fn perc_used(&self) -> f64 {
        let total_size = self.total_size();
        let max_size = self.max_size;
        let used_area = total_size.0 as f64 * total_size.1 as f64;
        let max_area = max_size.0 as f64 * max_size.1 as f64;

        if max_area == 0.0 {
            return 0.0;
        }

        (used_area / max_area) * 100.0
    }

    pub fn perc_free(&self) -> f64 {
        (1.0 - self.perc_used()) * 100.0
    }
}

pub(super) struct MegaTextureAtlas<ID> {
    pub layers: Vec<MegaTextureLayer<ID>>,
    pub texture_size: u32,
    pub max_depth: u32,
    pub padding: u32,
}

pub(super) struct MegaTextureLayer<ID> {
    pub entries: Vec<MegaTextureEntry<ID>>,
    pub packer: MaxRectsBin,
}

pub struct MegaTextureEntry<ID> {
    pub pixel_offset: (u32, u32),
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    pub image_data: ImageData,
    pub id: ID,
}

impl<ID> MegaTextureEntry<ID>
where
    ID: Clone,
{
    pub fn into_info(&self, index: MegaTextureIndex) -> MegaTextureEntryInfo<ID> {
        MegaTextureEntryInfo {
            pixel_offset: self.pixel_offset,
            uv_offset: self.uv_offset,
            uv_scale: self.uv_scale,
            id: self.id.clone(),
            index,
        }
    }
}

// Does not include ImageData
pub struct MegaTextureEntryInfo<ID> {
    pub pixel_offset: (u32, u32),
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    pub index: MegaTextureIndex,
    pub id: ID,
}

impl<ID> MegaTexture<ID>
where
    ID: std::hash::Hash + Eq + Clone + std::fmt::Debug,
{
    pub fn new(limits: &web_sys::GpuSupportedLimits, padding: u32) -> Self {
        let (texture_size, max_depth) = max_dimensions(limits);

        // let max_bindings_per_group = limits.max_sampled_textures_per_shader_stage();

        // These are some really interesting metrics, they let us know our wiggle room for textures
        // tracing::info!("Creating multi-atlas {}x{} w/ max depth: {} and max bindings per group: {}", texture_size, texture_size, max_depth, max_bindings_per_group);

        // tracing::info!("Total material image size per bind group: {}x{}",
        //     texture_size * max_depth * max_bindings_per_group,
        //     texture_size * max_depth * max_bindings_per_group,
        // );

        // tracing::info!("Absolute total material image size: {}x{}",
        //     texture_size * max_depth * max_bindings_per_group * limits.max_bind_groups(),
        //     texture_size * max_depth * max_bindings_per_group * limits.max_bind_groups(),
        // );

        Self {
            atlases: Vec::new(),
            texture_size,
            atlas_depth: max_depth,
            padding,
            mipmap: true,
            lookup: HashMap::new(),
        }
    }

    pub fn size(&self, limits: &web_sys::GpuSupportedLimits) -> MegaTextureSize {
        let inner_len: Vec<Vec<usize>> = self
            .atlases
            .iter()
            .map(|atlas| {
                atlas
                    .layers
                    .iter()
                    .map(|layer| layer.entries.len())
                    .collect()
            })
            .collect();

        let inner_size: Vec<Vec<Vec<(u32, u32)>>> = self
            .atlases
            .iter()
            .map(|atlas| {
                atlas
                    .layers
                    .iter()
                    .map(|layer| {
                        layer
                            .entries
                            .iter()
                            .map(|entry| entry.image_data.size())
                            .collect()
                    })
                    .collect()
            })
            .collect();

        let max_bindings_per_group = limits.max_sampled_textures_per_shader_stage();
        let max_bind_groups = limits.max_bind_groups();

        let max_size_per_bind_group = (
            self.texture_size * self.atlas_depth * max_bindings_per_group,
            self.texture_size * self.atlas_depth * max_bindings_per_group,
        );

        let max_size = (
            max_size_per_bind_group.0 * max_bind_groups,
            max_size_per_bind_group.1 * max_bind_groups,
        );

        MegaTextureSize {
            inner_len,
            texture_size: self.texture_size,
            max_depth: self.atlas_depth,
            inner_size,
            max_size,
            max_size_per_bind_group,
        }
    }

    pub fn get_index(&self, custom_id: &ID) -> Option<MegaTextureIndex> {
        self.lookup.get(custom_id).cloned()
    }

    pub fn get_entry(&self, custom_id: &ID) -> Option<&MegaTextureEntry<ID>> {
        self.get_index(custom_id).and_then(
            |MegaTextureIndex {
                 atlas_index,
                 layer_index,
                 entry_index,
             }| {
                self.atlases
                    .get(atlas_index)
                    .and_then(|atlas| atlas.layers.get(layer_index))
                    .and_then(|layer| layer.entries.get(entry_index))
            },
        )
    }

    pub fn get_entry_info(&self, custom_id: &ID) -> Option<MegaTextureEntryInfo<ID>> {
        let index = self.get_index(custom_id)?;
        self.get_entry(custom_id)
            .map(|entry| entry.into_info(index))
    }

    pub fn add_entries(
        &mut self,
        mut images: Vec<(ImageData, ID)>,
    ) -> Result<Vec<MegaTextureEntryInfo<ID>>> {
        if self.atlases.is_empty() {
            self.atlases.push(MegaTextureAtlas::new(
                self.texture_size,
                self.atlas_depth,
                self.padding,
            ));
        }

        let mut new_entries = Vec::new();

        loop {
            let atlas_index = self.atlases.len() - 1;

            let rejected_images = self.atlases.last_mut().unwrap().add_entries(
                &mut self.lookup,
                atlas_index,
                images,
                &mut new_entries,
            )?;

            if rejected_images.is_empty() {
                return Ok(new_entries);
            }

            // If we got rejected images, we need to create a new atlas (all layers are full up to max depth)
            images = rejected_images;

            self.atlases.push(MegaTextureAtlas::new(
                self.texture_size,
                self.atlas_depth,
                self.padding,
            ));
        }
    }
}

impl<ID> MegaTextureAtlas<ID>
where
    ID: std::hash::Hash + Eq + Clone + std::fmt::Debug,
{
    pub fn new(texture_size: u32, max_depth: u32, padding: u32) -> Self {
        Self {
            layers: Vec::new(),
            texture_size,
            max_depth,
            padding,
        }
    }

    // return is the rejected images that could not be placed in the atlas due to max depth
    pub fn add_entries(
        &mut self,
        lookup: &mut HashMap<ID, MegaTextureIndex>,
        atlas_index: usize,
        images: Vec<(ImageData, ID)>,
        new_entries: &mut Vec<MegaTextureEntryInfo<ID>>,
    ) -> Result<Vec<(ImageData, ID)>> {
        if images.is_empty() {
            return Ok(images);
        }

        // allows us to have a stable index and mutable vec that we can take from
        let mut images: Vec<Option<(ImageData, ID)>> = images.into_iter().map(Some).collect();

        let padding = self.padding as i32;
        let padding_width_x2 = padding * 2;
        let padding_height_x2 = padding * 2;

        let mut items_to_place: Vec<Dimension> = images
            .iter()
            .enumerate()
            .map(|(index, image)| {
                let (width, height) = image.as_ref().unwrap().0.size();
                Dimension::with_id(
                    index as isize,
                    width as i32 + padding_width_x2,
                    height as i32 + padding_height_x2,
                    0,
                )
            })
            .collect();

        if self.layers.is_empty() {
            self.layers
                .push(MegaTextureLayer::new(self.texture_size, self.texture_size));
        }

        loop {
            let layer_index = self.layers.len() - 1;
            let current_layer = self.layers.last_mut().unwrap();
            let atlas_width = self.texture_size as i32;
            let atlas_height = self.texture_size as i32;

            let (inserted, rejected) = current_layer
                .packer
                .insert_list(&items_to_place, Heuristic::BestAreaFit);

            if inserted.is_empty() && !items_to_place.is_empty() && current_layer.entries.is_empty()
            {
                let (largest_image_width, largest_image_height) =
                    items_to_place.iter().fold((0, 0), |(max_w, max_h), dim| {
                        (
                            max_w.max(dim.width() as usize),
                            max_h.max(dim.height() as usize),
                        )
                    });

                return Err(AwsmCoreError::MegaTextureAtlasSize {
                    largest_img_width: largest_image_width - (padding_width_x2 as usize),
                    largest_img_height: largest_image_height - (padding_height_x2 as usize),
                    atlas_width: atlas_width as usize,
                    atlas_height: atlas_height as usize,
                    padding: padding as usize,
                });
            }

            for rect in inserted.into_iter() {
                let (image_data, id) = images[rect.id() as usize].take().unwrap();

                let index = MegaTextureIndex {
                    atlas_index,
                    layer_index,
                    entry_index: current_layer.entries.len(),
                };

                if lookup.insert(id.clone(), index).is_some() {
                    return Err(AwsmCoreError::MegaTextureDuplicateId {
                        id: format!("{id:?}"),
                    });
                }

                let (img_width, img_height) = image_data.size();
                let pixel_offset = (rect.x() + padding, rect.y() + padding);

                let entry = MegaTextureEntry {
                    pixel_offset: (pixel_offset.0 as u32, pixel_offset.1 as u32),
                    uv_offset: [
                        pixel_offset.0 as f32 / atlas_width as f32,
                        pixel_offset.1 as f32 / atlas_height as f32,
                    ],
                    uv_scale: [
                        img_width as f32 / atlas_width as f32,
                        img_height as f32 / atlas_height as f32,
                    ],
                    id,
                    image_data,
                };

                new_entries.push((&entry).into_info(index));

                current_layer.entries.push(entry);
            }

            if rejected.is_empty() {
                // finished!
                break;
            }

            if self.layers.len() as u32 >= self.max_depth {
                let rejected_images: Vec<(ImageData, ID)> = rejected
                    .into_iter()
                    .filter_map(|dim| images[dim.id() as usize].take())
                    .collect();

                return Ok(rejected_images);
            }

            self.layers.push(MegaTextureLayer::new(
                atlas_width as u32,
                atlas_height as u32,
            ));
            items_to_place = rejected;
        }

        Ok(Vec::new())
    }
}

impl<ID> MegaTextureLayer<ID> {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            entries: Vec::new(),
            packer: MaxRectsBin::new(width as i32, height as i32),
        }
    }
}

fn max_dimensions(limits: &web_sys::GpuSupportedLimits) -> (u32, u32) {
    let max_dimension_2d = limits.max_texture_dimension_2d();
    let max_depth_2d_array = limits.max_texture_array_layers();
    let max_memory = limits.max_buffer_size();

    // Rgba16Float = 4 channels * 2 bytes per channel = 8 bytes per pixel
    let bytes_per_pixel = 8u32;

    let mut texture_size = max_dimension_2d;

    loop {
        if ((texture_size * texture_size) * bytes_per_pixel) as f64 <= max_memory {
            break;
        }
        tracing::warn!(
            "Max texture size {}x{} exceeds max buffer size {}, reducing to {}x{}",
            texture_size,
            texture_size,
            max_memory,
            texture_size / 2,
            texture_size / 2
        );
        texture_size /= 2;
    }

    let memory_per_texture = texture_size * texture_size * bytes_per_pixel;
    let max_depth_by_memory = (max_memory / memory_per_texture as f64).floor() as u32;

    let max_depth = max_depth_2d_array.min(max_depth_by_memory);

    (texture_size, max_depth)
}
