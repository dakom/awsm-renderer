use std::cell::RefCell;
use std::collections::HashMap;

use crate::bind_groups::{BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupLayoutResource, SamplerBindingLayout, SamplerBindingType, StorageTextureAccess, StorageTextureBindingLayout, TextureBindingLayout};
use crate::pipeline::layout::{PipelineLayoutDescriptor, PipelineLayoutKind};
use crate::pipeline::{ComputePipelineDescriptor, ProgrammableStage};
use crate::shaders::ShaderModuleExt;

use crate::texture::{TextureFormat, TextureSampleType};
use crate::sampler::{FilterMode, SamplerDescriptor};
use crate::{bind_groups::{BindGroupDescriptor, BindGroupEntry, BindGroupResource}, command::compute_pass::ComputePassDescriptor, error::AwsmCoreError, renderer::AwsmRendererWebGpu, shaders::ShaderModuleDescriptor, texture::{TextureViewDescriptor, TextureViewDimension}};
use crate::error::Result;

#[derive(Debug, Clone)]
struct MipmapPipeline {
    pub compute_pipeline: web_sys::GpuComputePipeline,
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
}

thread_local! {
    // key is TextureFormat as u32
    static MIPMAP_PIPELINE: RefCell<HashMap<u32, MipmapPipeline>> = RefCell::new(HashMap::new());
}

pub async fn generate_mipmaps(
    gpu: &AwsmRendererWebGpu,
    texture: &web_sys::GpuTexture,
    mut current_width: u32,
    mut current_height: u32,
    mip_levels: u32,
) -> Result<()> {
    let MipmapPipeline {compute_pipeline, bind_group_layout} = get_mipmap_pipeline(gpu, texture.format()).await?;
    
    // Create a linear sampler for mipmap generation
    let sampler_descriptor = SamplerDescriptor {
        min_filter: Some(FilterMode::Linear),
        mag_filter: Some(FilterMode::Linear),
        ..Default::default()
    };
    let sampler = gpu.create_sampler(Some(&sampler_descriptor.into()));
    
    let command_encoder = gpu.create_command_encoder(Some("Generate Mipmaps"));
    
    for mip_level in 1..mip_levels {
        let next_width = (current_width / 2).max(1);
        let next_height = (current_height / 2).max(1);
        
        // Create texture views for input (previous mip) and output (current mip)
        let input_view_descriptor = TextureViewDescriptor::new(Some("Input Mipmap View"))
            .with_base_mip_level(mip_level - 1)
            .with_mip_level_count(1);
        let input_view = texture.create_view_with_descriptor(&input_view_descriptor.into()).map_err(AwsmCoreError::create_texture_view)?;
        
        let output_view_descriptor = TextureViewDescriptor::new(Some("Output Mipmap View"))
            .with_base_mip_level(mip_level)
            .with_mip_level_count(1)
            .with_dimension(TextureViewDimension::N2d);
        let output_view = texture.create_view_with_descriptor(&output_view_descriptor.into()).map_err(AwsmCoreError::create_texture_view)?;
        
        // Create bind group
        // Input texture binding
        let input_binding = BindGroupEntry::new(0, BindGroupResource::TextureView(&input_view));
        
        // Sampler binding
        let sampler_binding = BindGroupEntry::new(1, BindGroupResource::Sampler(&sampler));
        
        // Output texture binding
        let output_binding = BindGroupEntry::new(2, BindGroupResource::TextureView(&output_view)); 
        
        let bind_group = gpu.create_bind_group(&BindGroupDescriptor::new(
            &bind_group_layout,
            Some("Mipmap Bind Group"),
            vec![input_binding, sampler_binding, output_binding],
        ).into());
        
        // Dispatch compute shader
        let compute_pass = command_encoder.begin_compute_pass(Some(&ComputePassDescriptor::new(Some("Mipmap Compute Pass")).into()));
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, None)?;
        
        let workgroup_size_x = (next_width + 7) / 8; // Ceiling division
        let workgroup_size_y = (next_height + 7) / 8;
        compute_pass.dispatch_workgroups(workgroup_size_x, Some(workgroup_size_y), Some(1));
        compute_pass.end();
        
        current_width = next_width;
        current_height = next_height;
    }
    
    // Submit the commands
    let command_buffer = command_encoder.finish();
    gpu.submit_commands(&command_buffer);

    Ok(())
}

async fn get_mipmap_pipeline(gpu: &AwsmRendererWebGpu, format: TextureFormat) -> Result<MipmapPipeline> {
    let pipeline = MIPMAP_PIPELINE.with(|pipeline_cell| {
        pipeline_cell.borrow().get(&(format as u32)).cloned()
    });

    if let Some(pipeline) = pipeline {
        return Ok(pipeline);
    }

    let shader_module = gpu.compile_shader(&ShaderModuleDescriptor::new(
        include_str!("./mipmap_shader.wgsl"),
        Some("Mipmap Shader")
    ).into());

    shader_module.validate_shader().await?;

    let compute = ProgrammableStage::new(shader_module, None); 

    let bind_group_layout = gpu.create_bind_group_layout(&BindGroupLayoutDescriptor::new(Some("Mipmap Bind Group Layout"))
            .with_entries(vec![
                BindGroupLayoutEntry::new(0, 
                    BindGroupLayoutResource::Texture(TextureBindingLayout::new()
                        .with_sample_type(TextureSampleType::Float)
                        .with_view_dimension(TextureViewDimension::N2d)
                        .with_multisampled(false)
                    )
                )
                .with_visibility_compute(),
                BindGroupLayoutEntry::new(1,
                    BindGroupLayoutResource::Sampler(
                        SamplerBindingLayout::new()
                            .with_binding_type(SamplerBindingType::Filtering)
                    )
                )
                .with_visibility_compute(),
                BindGroupLayoutEntry::new(2, 
                    BindGroupLayoutResource::StorageTexture(StorageTextureBindingLayout::new(format)
                        .with_view_dimension(TextureViewDimension::N2d)
                        .with_access(StorageTextureAccess::WriteOnly))
                )
                .with_visibility_compute()
            ])
            .into()
    )?;
    let layout = PipelineLayoutKind::Custom(gpu.create_pipeline_layout(&PipelineLayoutDescriptor::new(Some("Mipmap Pipeline Layout"), 
        vec![bind_group_layout.clone()]
    ).into()));

    let pipeline_descriptor = ComputePipelineDescriptor::new(compute, layout.clone(), Some("Mipmap Pipeline"));

    // UGH - move the whole thing out of the closure... let async infect everything... look at earlier async comment 
    let pipeline = gpu.create_compute_pipeline(&pipeline_descriptor.into()).await?;

    MIPMAP_PIPELINE.with(|pipeline_cell| {
        let pipeline = MipmapPipeline {
            compute_pipeline: pipeline,
            bind_group_layout
        };
        pipeline_cell.borrow_mut().insert(format as u32, pipeline.clone());
        Ok(pipeline)
    })

}

pub fn calculate_mipmap_levels(
    width: u32,
    height: u32,
) -> u32 {
    ((width.max(height) as f32).log2().floor() as u32) + 1
}