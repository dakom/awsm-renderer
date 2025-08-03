use std::{borrow::Cow, cell::RefCell};

use crate::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
        BindGroupLayoutResource, BindGroupResource, StorageTextureAccess,
        StorageTextureBindingLayout, TextureBindingLayout,
    },
    command::compute_pass::ComputePassDescriptor,
    error::{AwsmCoreError, Result},
    pipeline::{
        layout::{PipelineLayoutDescriptor, PipelineLayoutKind},
        ComputePipelineDescriptor, ProgrammableStage,
    },
    renderer::AwsmRendererWebGpu,
    shaders::{ShaderModuleDescriptor, ShaderModuleExt},
    texture::{
        Extent3d, TextureDescriptor, TextureFormat, TextureSampleType, TextureUsage,
        TextureViewDescriptor, TextureViewDimension,
    },
};
use binpack2d::{
    maxrects::{Heuristic, MaxRectsBin},
    Dimension,
};

use crate::image::ImageData;

thread_local! {
    // key is TextureFormat as u32
    static ATLAS_PIPELINE: RefCell<Option<AtlasPipeline>> = RefCell::new(None);
    static ATLAS_SHADER_MODULE: RefCell<Option<web_sys::GpuShaderModule>> = RefCell::new(None);
}

pub struct ImageAtlas {
    pub layers: Vec<ImageAtlasLayer>,
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

impl ImageAtlas {
    pub fn new(width: u32, height: u32, padding: u32) -> Self {
        Self {
            layers: vec![ImageAtlasLayer::new(width, height, padding)],
        }
    }

    // second param is an optional custom id that can be used to identify the image in the atlas
    pub fn add_entries(&mut self, images: Vec<(ImageData, Option<u64>)>) -> Result<()> {
        if images.is_empty() {
            return Ok(());
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

            self.layers.push(ImageAtlasLayer::new(
                atlas_width as u32,
                atlas_height as u32,
                padding as u32,
            ));
            items_to_place = rejected;
        }

        Ok(())
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

    pub async fn write_texture_array(
        &self,
        gpu: &AwsmRendererWebGpu,
        depth: Option<usize>,
    ) -> Result<web_sys::GpuTexture> {
        let width = self.layers.first().map_or(0, |layer| layer.width);
        let height = self.layers.first().map_or(0, |layer| layer.height);

        let depth = match depth {
            // allocate double what we need
            None => self.layers.len() as u32 * 2,
            Some(depth) => {
                if depth < self.layers.len() {
                    return Err(AwsmCoreError::ImageAtlasDepthTooSmall {
                        required: self.layers.len(),
                        provided: depth,
                    });
                }
                depth as u32
            }
        };

        let dest_tex_array = gpu.create_texture(
            &TextureDescriptor::new(
                TextureFormat::Rgba16float,
                Extent3d::new(width, Some(height), Some(depth)),
                TextureUsage::new().with_storage_binding(),
            )
            .into(),
        )?;

        let dest_texture_view = dest_tex_array
            .create_view_with_descriptor(
                &TextureViewDescriptor::new(Some("Atlas Dest Texture View"))
                    .with_dimension(TextureViewDimension::N2dArray)
                    .with_array_layer_count(depth)
                    .into(),
            )
            .map_err(AwsmCoreError::create_texture_view)?;

        for (index, layer) in self.layers.iter().enumerate() {
            layer
                .write_texture_to_array(gpu, &dest_texture_view, index as u32)
                .await?;
        }

        Ok(dest_tex_array)
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

    pub async fn write_texture_to_array(
        &self,
        gpu: &AwsmRendererWebGpu,
        dest_texture_view: &web_sys::GpuTextureView,
        layer_index: u32,
    ) -> Result<()> {
        let atlas_pipelines = get_atlas_pipeline(gpu).await?;
        let command_encoder = gpu.create_command_encoder(Some("Write Texture Atlas Layer"));
        let padding_x2 = self.padding * 2;

        for entry in self.entries.iter() {
            let texture = entry.image_data.create_texture(gpu, None, false).await?;
            let texture_view = texture
                .create_view()
                .map_err(AwsmCoreError::create_texture_view)?;

            // Dispatch compute shader
            let compute_pass = command_encoder.begin_compute_pass(Some(
                &ComputePassDescriptor::new(Some("Atlas Compute Pass")).into(),
            ));

            let bind_group = gpu.create_bind_group(
                &BindGroupDescriptor::new(
                    &atlas_pipelines.bind_group_layout,
                    Some("Atlas"),
                    vec![
                        BindGroupEntry::new(
                            0,
                            BindGroupResource::TextureView(Cow::Owned(texture_view)),
                        ),
                        BindGroupEntry::new(
                            1,
                            BindGroupResource::TextureView(Cow::Borrowed(dest_texture_view)),
                        ),
                    ],
                )
                .into(),
            );

            compute_pass.set_pipeline(&atlas_pipelines.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, None)?;

            let (image_width, image_height) = entry.image_data.size();
            let workgroup_size_x = (image_width + padding_x2).div_ceil(8);
            let workgroup_size_y = (image_height + padding_x2).div_ceil(8);
            compute_pass.dispatch_workgroups(workgroup_size_x, Some(workgroup_size_y), Some(1));
            compute_pass.end();
        }

        let command_buffer = command_encoder.finish();
        gpu.submit_commands(&command_buffer);

        Ok(())
    }
}

#[derive(Clone)]
struct AtlasPipeline {
    pub compute_pipeline: web_sys::GpuComputePipeline,
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
}

async fn get_atlas_pipeline(gpu: &AwsmRendererWebGpu) -> Result<AtlasPipeline> {
    let pipeline = ATLAS_PIPELINE.with(|pipeline_cell| pipeline_cell.borrow().clone());

    if let Some(pipeline) = pipeline {
        return Ok(pipeline);
    }

    let shader_module = ATLAS_SHADER_MODULE.with(|shader_module| shader_module.borrow().clone());

    let shader_module = match shader_module {
        Some(module) => module,
        None => {
            let shader_module = gpu.compile_shader(
                &ShaderModuleDescriptor::new(
                    include_str!("./atlas/atlas_shader.wgsl"),
                    Some("Atlas Shader"),
                )
                .into(),
            );

            shader_module.validate_shader().await?;

            ATLAS_SHADER_MODULE.with(|shader_module_rc| {
                *shader_module_rc.borrow_mut() = Some(shader_module.clone());
            });

            shader_module
        }
    };

    let compute = ProgrammableStage::new(&shader_module, None);

    let bind_group_layout = gpu.create_bind_group_layout(
        &BindGroupLayoutDescriptor::new(Some("Atlas Bind Group Layout"))
            .with_entries(vec![
                BindGroupLayoutEntry::new(
                    0,
                    BindGroupLayoutResource::Texture(
                        TextureBindingLayout::new()
                            .with_sample_type(TextureSampleType::Float)
                            .with_view_dimension(TextureViewDimension::N2d),
                    ),
                )
                .with_visibility_compute(),
                BindGroupLayoutEntry::new(
                    2,
                    BindGroupLayoutResource::StorageTexture(
                        StorageTextureBindingLayout::new(TextureFormat::Rgba16float)
                            .with_view_dimension(TextureViewDimension::N2d)
                            .with_access(StorageTextureAccess::WriteOnly),
                    ),
                )
                .with_visibility_compute(),
            ])
            .into(),
    )?;

    let layout = gpu.create_pipeline_layout(
        &PipelineLayoutDescriptor::new(
            Some("Atlas Pipeline Layout"),
            vec![bind_group_layout.clone()],
        )
        .into(),
    );

    let layout = PipelineLayoutKind::Custom(&layout);

    let pipeline_descriptor =
        ComputePipelineDescriptor::new(compute, layout.clone(), Some("Atlas Pipeline"));

    let pipeline = gpu
        .create_compute_pipeline(&pipeline_descriptor.into())
        .await?;

    ATLAS_PIPELINE.with(|pipeline_cell| {
        let pipeline = AtlasPipeline {
            compute_pipeline: pipeline,
            bind_group_layout,
        };
        *pipeline_cell.borrow_mut() = Some(pipeline.clone());
        Ok(pipeline)
    })
}
