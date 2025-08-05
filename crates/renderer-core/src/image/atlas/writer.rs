use std::{borrow::Cow, cell::RefCell};

use crate::{bind_groups::{BindGroupDescriptor, BindGroupEntry, BindGroupResource}, buffers::{BufferBinding, BufferDescriptor, BufferUsage}, command::compute_pass::ComputePassDescriptor, image::atlas::{pipeline::get_atlas_pipeline, ImageAtlas, ImageAtlasLayer}, renderer::AwsmRendererWebGpu, texture::{Extent3d, TextureDescriptor, TextureFormat, TextureUsage, TextureViewDescriptor, TextureViewDimension}};
use crate::error::{AwsmCoreError, Result};

thread_local! {
    // key is TextureFormat as u32
    static UNIFORM_BUFFER: RefCell<Option<web_sys::GpuBuffer>> = RefCell::new(None);
}

impl ImageAtlas {
    pub async fn write_texture_array(
        &self,
        gpu: &AwsmRendererWebGpu,
    ) -> Result<web_sys::GpuTexture> {
        let width = self.layers.first().map_or(0, |layer| layer.width);
        let height = self.layers.first().map_or(0, |layer| layer.height);

        let dest_tex_array = gpu.create_texture(
            &TextureDescriptor::new(
                TextureFormat::Rgba16float,
                Extent3d::new(width, Some(height), Some(self.layers.len() as u32)),
                TextureUsage::new().with_storage_binding(),
            )
            .into(),
        )?;

        let dest_texture_view = dest_tex_array
            .create_view_with_descriptor(
                &TextureViewDescriptor::new(Some("Atlas Dest Texture View"))
                    .with_dimension(TextureViewDimension::N2dArray)
                    .with_array_layer_count(self.layers.len() as u32)
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
    pub async fn write_texture_to_array(
        &self,
        gpu: &AwsmRendererWebGpu,
        dest_texture_view: &web_sys::GpuTextureView,
        layer_index: u32,
    ) -> Result<()> {
        let atlas_pipelines = get_atlas_pipeline(gpu).await?;
        let command_encoder = gpu.create_command_encoder(Some("Write Texture Atlas Layer"));
        let padding_x2 = self.padding * 2;
        let mut textures = Vec::new();


        for entry in self.entries.iter() {
            let texture = entry.image_data.create_texture(gpu, None, false).await?;
            let texture_view = texture
                .create_view()
                .map_err(AwsmCoreError::create_texture_view)?;

            // Dispatch compute shader
            let compute_pass = command_encoder.begin_compute_pass(Some(
                &ComputePassDescriptor::new(Some("Atlas Compute Pass")).into(),
            ));


            let needs_create = UNIFORM_BUFFER.with(|buffer_cell| buffer_cell.borrow().is_none());

            if needs_create {
                let uniform_buffer = gpu.create_buffer(
                    &BufferDescriptor::new(
                        Some("Atlas Uniform Buffer"),
                        16,
                        BufferUsage::new().with_uniform().with_copy_dst()
                    )
                    .into(),
                )?;


                UNIFORM_BUFFER.with(move |buffer_cell| {
                    *buffer_cell.borrow_mut() = Some(uniform_buffer);
                });
            }

            let uniform_buffer = UNIFORM_BUFFER.with(|buffer_cell| {
                buffer_cell.borrow().clone().unwrap()
            });

            let entry_data = [
                entry.pixel_offset.0 as f32,
                entry.pixel_offset.1 as f32,
                self.padding as f32,
                layer_index as f32,
            ];
            let uniform_data:[u8; 16] = entry_data
                .iter()
                .flat_map(|&f| f.to_ne_bytes())
                .collect::<Vec<u8>>()
                .try_into()
                .expect("Failed to convert entry data to uniform data");
            gpu.write_buffer(&uniform_buffer, None, uniform_data.as_slice(), None, None)?;


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
                        BindGroupEntry::new(
                            2,
                            BindGroupResource::Buffer(BufferBinding::new(&uniform_buffer)
                                .with_size(16)),
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

            textures.push(texture);
        }

        let command_buffer = command_encoder.finish();
        gpu.submit_commands(&command_buffer);

        for texture in textures { 
            texture.destroy();
        }

        Ok(())
    }
}
