pub mod pipeline;
pub mod writer;

use crate::error::{AwsmCoreError, Result};
use binpack2d::{
    maxrects::{Heuristic, MaxRectsBin},
    Dimension,
};

use crate::image::ImageData;

pub struct MultiImageAtlas {
    pub atlases: Vec<ImageAtlas>,
    pub texture_size: u32,
    pub max_depth_per_atlas: u32,
    pub padding: u32,
    pub max_bindings_per_group: u32
}

pub struct ImageAtlas {
    pub layers: Vec<ImageAtlasLayer>,
    pub max_depth: Option<u32>,
}

pub struct ImageAtlasLayer {
    pub entries: Vec<ImageAtlasEntry>,
    pub packer: MaxRectsBin,
    pub width: u32,
    pub height: u32,
    pub padding: u32,
}

pub struct ImageAtlasEntry {
    pub pixel_offset: (u32, u32),
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    pub image_data: ImageData,
    pub custom_id: Option<u64>,
}

impl MultiImageAtlas {
    pub fn new(limits: &web_sys::GpuSupportedLimits, padding: u32) -> Self {
        let (texture_size, max_depth) = max_dimensions(limits);

        let max_bindings_per_group = limits.max_sampled_textures_per_shader_stage();

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
            atlases: vec![ImageAtlas::new(texture_size, texture_size, padding, Some(max_depth))],
            texture_size,
            max_depth_per_atlas: max_depth,
            padding,
            max_bindings_per_group
        }
    }

    pub fn add_entries(&mut self, mut images: Vec<(ImageData, Option<u64>)>) -> Result<()> {
        loop {
            let rejected_images = self
                .atlases
                .last_mut()
                .unwrap()
                .add_entries(images)?;

            if rejected_images.is_empty() {
                return Ok(());
            }

            images = rejected_images;

            self.atlases.push(ImageAtlas::new(
                self.texture_size,
                self.texture_size,
                self.padding,
                Some(self.max_depth_per_atlas),
            ));

        }
    }

    // returns atlas_index, layer_index and entry_index
    pub fn find_custom_id_index(&self, custom_id: u64) -> Option<(usize, usize, usize)> {
        for (atlas_index, atlas) in self.atlases.iter().enumerate() {
            if let Some((layer_index, entry_index)) = atlas.find_custom_id_index(custom_id) {
                return Some((atlas_index, layer_index, entry_index));
            }
        }
        None
    }
}


impl ImageAtlas {
    pub fn new(width: u32, height: u32, padding: u32, max_depth: Option<u32>) -> Self {
        Self {
            layers: vec![ImageAtlasLayer::new(width, height, padding)],
            max_depth
        }
    }

    // second param is an optional custom id that can be used to identify the image in the atlas
    // return is the rejected images that could not be placed in the atlas due to max depth
    pub fn add_entries(&mut self, images: Vec<(ImageData, Option<u64>)>) -> Result<Vec<(ImageData, Option<u64>)>> {
        if images.is_empty() {
            return Ok(images);
        }

        // allows us to have a stable index and mutable vec that we can take from
        let mut images: Vec<Option<(ImageData, Option<u64>)>> =
            images.into_iter().map(Some).collect();

        let padding = self.layers.first().as_ref().unwrap().padding as i32;
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

        loop {
            let current_layer = self.layers.last_mut().unwrap();
            let atlas_width = current_layer.width as i32;
            let atlas_height = current_layer.height as i32;

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

                return Err(AwsmCoreError::ImageAtlasSize {
                    largest_img_width: largest_image_width - (padding_width_x2 as usize),
                    largest_img_height: largest_image_height - (padding_height_x2 as usize),
                    atlas_width: atlas_width as usize,
                    atlas_height: atlas_height as usize,
                    padding: padding as usize,
                });
            }

            current_layer
                .entries
                .extend(inserted.into_iter().map(|rect| {
                    let (image_data, custom_id) = images[rect.id() as usize].take().unwrap();
                    let (img_width, img_height) = image_data.size();
                    let pixel_offset = (rect.x() + padding, rect.y() + padding);

                    ImageAtlasEntry {
                        pixel_offset: (pixel_offset.0 as u32, pixel_offset.1 as u32),
                        uv_offset: [
                            pixel_offset.0 as f32 / atlas_width as f32,
                            pixel_offset.1 as f32 / atlas_height as f32,
                        ],
                        uv_scale: [
                            img_width as f32 / atlas_width as f32,
                            img_height as f32 / atlas_height as f32,
                        ],
                        custom_id,
                        image_data,
                    }
                }));

            if rejected.is_empty() {
                // finished!
                break;
            }

            if let Some(max_depth) = self.max_depth {
                if self.layers.len() as u32 >= max_depth {
                    let rejected_images: Vec<(ImageData, Option<u64>)> = rejected
                        .into_iter()
                        .filter_map(|dim| images[dim.id() as usize].take())
                        .collect();

                    return Ok(rejected_images);
                }
            }

            self.layers.push(ImageAtlasLayer::new(
                atlas_width as u32,
                atlas_height as u32,
                padding as u32,
            ));
            items_to_place = rejected;
        }

        Ok(Vec::new())
    }

    // returns layer_index and entry_index
    pub fn find_custom_id_index(&self, custom_id: u64) -> Option<(usize, usize)> {
        for (layer_index, layer) in self.layers.iter().enumerate() {
            for (entry_index, entry) in layer.entries.iter().enumerate() {
                if entry.custom_id == Some(custom_id) {
                    return Some((layer_index, entry_index));
                }
            }
        }
        None
    }

}

impl ImageAtlasLayer {
    pub fn new(width: u32, height: u32, padding: u32) -> Self {
        Self {
            entries: Vec::new(),
            packer: MaxRectsBin::new(width as i32, height as i32),
            width,
            height,
            padding,
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
            texture_size,texture_size,
            max_memory,
            texture_size / 2,texture_size / 2
        );
        texture_size /= 2;
    }

    let memory_per_texture = texture_size * texture_size * bytes_per_pixel;
    let max_depth_by_memory = (max_memory / memory_per_texture as f64).floor() as u32;

    let max_depth = max_depth_2d_array.min(max_depth_by_memory); 

    (texture_size, max_depth) 
}