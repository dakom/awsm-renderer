use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::bind_groups::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupLayoutResource, SamplerBindingLayout,
    SamplerBindingType, StorageTextureAccess, StorageTextureBindingLayout, TextureBindingLayout,
};
use crate::pipeline::layout::{PipelineLayoutDescriptor, PipelineLayoutKind};
use crate::pipeline::{ComputePipelineDescriptor, ProgrammableStage};
use crate::shaders::ShaderModuleExt;

use crate::error::Result;
use crate::sampler::{FilterMode, SamplerDescriptor};
use crate::texture::{TextureFormat, TextureSampleType};
use crate::{
    bind_groups::{BindGroupDescriptor, BindGroupEntry, BindGroupResource},
    command::compute_pass::ComputePassDescriptor,
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
    shaders::ShaderModuleDescriptor,
    texture::{TextureViewDescriptor, TextureViewDimension},
};

#[derive(Debug, Clone)]
struct MipmapPipeline {
    pub compute_pipeline: web_sys::GpuComputePipeline,
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
}

thread_local! {
    static MIPMAP_PIPELINE: RefCell<HashMap<LookupKey, MipmapPipeline>> = RefCell::new(HashMap::new());
}

#[derive(Hash, Debug, Eq, PartialEq)]
struct LookupKey {
    pub texture_format: String,
    pub is_array: bool,
}

fn storage_format_to_wgsl(format: TextureFormat) -> Result<&'static str> {
    match format {
        TextureFormat::Rgba8unorm => Ok("rgba8unorm"),
        TextureFormat::Rgba16float => Ok("rgba16float"),
        TextureFormat::Rgba32float => Ok("rgba32float"),
        _ => Err(AwsmCoreError::MipmapUnsupportedFormat(format)),
    }
}

fn mipmap_shader_source(format: TextureFormat, is_array: bool) -> Result<String> {
    let storage_format = storage_format_to_wgsl(format)?;

    if is_array {
        Ok(format!(
            r#"
@group(0) @binding(0) var input_texture: texture_2d_array<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var output_texture: texture_storage_2d_array<{storage_format}, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let output_size = textureDimensions(output_texture);

    if (global_id.x >= output_size.x || global_id.y >= output_size.y) {{
        return;
    }}

    let layer = i32(global_id.z);
    let input_size = textureDimensions(input_texture);

    let input_coord = vec2<f32>(global_id.xy) * 2.0 + 0.5;
    let base_uv = input_coord / vec2<f32>(input_size.xy);
    let texel_size = 1.0 / vec2<f32>(input_size.xy);

    let sample_00 = textureSampleLevel(input_texture, input_sampler, base_uv + vec2<f32>(-0.5, -0.5) * texel_size, layer, 0.0);
    let sample_01 = textureSampleLevel(input_texture, input_sampler, base_uv + vec2<f32>(0.5, -0.5) * texel_size, layer, 0.0);
    let sample_10 = textureSampleLevel(input_texture, input_sampler, base_uv + vec2<f32>(-0.5, 0.5) * texel_size, layer, 0.0);
    let sample_11 = textureSampleLevel(input_texture, input_sampler, base_uv + vec2<f32>(0.5, 0.5) * texel_size, layer, 0.0);

    let result = (sample_00 + sample_01 + sample_10 + sample_11) * 0.25;

    textureStore(output_texture, vec2<i32>(global_id.xy), layer, result);
}}
"#
        ))
    } else {
        Ok(format!(
            r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var output_texture: texture_storage_2d<{storage_format}, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let output_size = textureDimensions(output_texture);

    if (global_id.x >= output_size.x || global_id.y >= output_size.y) {{
        return;
    }}

    let input_size = textureDimensions(input_texture, 0);

    let input_coord = vec2<f32>(global_id.xy) * 2.0 + 0.5;
    let base_uv = input_coord / vec2<f32>(input_size);
    let texel_size = 1.0 / vec2<f32>(input_size);

    let sample_00 = textureSampleLevel(input_texture, input_sampler, base_uv + vec2<f32>(-0.5, -0.5) * texel_size, 0.0);
    let sample_01 = textureSampleLevel(input_texture, input_sampler, base_uv + vec2<f32>(0.5, -0.5) * texel_size, 0.0);
    let sample_10 = textureSampleLevel(input_texture, input_sampler, base_uv + vec2<f32>(-0.5, 0.5) * texel_size, 0.0);
    let sample_11 = textureSampleLevel(input_texture, input_sampler, base_uv + vec2<f32>(0.5, 0.5) * texel_size, 0.0);

    let result = (sample_00 + sample_01 + sample_10 + sample_11) * 0.25;

    textureStore(output_texture, vec2<i32>(global_id.xy), result);
}}
"#
        ))
    }
}

pub async fn generate_mipmaps(
    gpu: &AwsmRendererWebGpu,
    texture: &web_sys::GpuTexture,
    mut current_width: u32,
    mut current_height: u32,
    array_layers: u32,
    is_array: bool,
    mip_levels: u32,
) -> Result<()> {
    let MipmapPipeline {
        compute_pipeline,
        bind_group_layout,
    } = get_mipmap_pipeline(gpu, texture.format(), is_array).await?;

    // Create a linear sampler for smooth filtering
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

        // Determine the appropriate view dimension based on array_layers
        let view_dimension = if is_array {
            TextureViewDimension::N2dArray
        } else {
            TextureViewDimension::N2d
        };

        // Create texture views for input (previous mip) and output (current mip)
        let mut input_view_descriptor = TextureViewDescriptor::new(Some("Input Mipmap View"))
            .with_base_mip_level(mip_level - 1)
            .with_dimension(view_dimension)
            .with_mip_level_count(1);
        if is_array {
            input_view_descriptor = input_view_descriptor.with_array_layer_count(array_layers);
        }
        let input_view = texture
            .create_view_with_descriptor(&input_view_descriptor.into())
            .map_err(AwsmCoreError::create_texture_view)?;

        let mut output_view_descriptor = TextureViewDescriptor::new(Some("Output Mipmap View"))
            .with_base_mip_level(mip_level)
            .with_dimension(view_dimension)
            .with_mip_level_count(1);
        if is_array {
            output_view_descriptor = output_view_descriptor.with_array_layer_count(array_layers);
        }
        let output_view = texture
            .create_view_with_descriptor(&output_view_descriptor.into())
            .map_err(AwsmCoreError::create_texture_view)?;

        // Create bind group
        // Input texture binding
        let input_binding =
            BindGroupEntry::new(0, BindGroupResource::TextureView(Cow::Owned(input_view)));

        // Sampler binding
        let sampler_binding = BindGroupEntry::new(1, BindGroupResource::Sampler(&sampler));

        // Output texture binding
        let output_binding =
            BindGroupEntry::new(2, BindGroupResource::TextureView(Cow::Owned(output_view)));

        let bind_group = gpu.create_bind_group(
            &BindGroupDescriptor::new(
                &bind_group_layout,
                Some("Mipmap Bind Group"),
                vec![input_binding, sampler_binding, output_binding],
            )
            .into(),
        );

        // Dispatch compute shader
        let compute_pass = command_encoder.begin_compute_pass(Some(
            &ComputePassDescriptor::new(Some("Mipmap Compute Pass")).into(),
        ));
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, None)?;

        let workgroup_size_x = next_width.div_ceil(8);
        let workgroup_size_y = next_height.div_ceil(8);
        compute_pass.dispatch_workgroups(
            workgroup_size_x,
            Some(workgroup_size_y),
            Some(array_layers),
        );
        compute_pass.end();

        current_width = next_width;
        current_height = next_height;
    }

    // Submit the commands
    let command_buffer = command_encoder.finish();
    gpu.submit_commands(&command_buffer);

    Ok(())
}

async fn get_mipmap_pipeline(
    gpu: &AwsmRendererWebGpu,
    format: TextureFormat,
    is_array: bool,
) -> Result<MipmapPipeline> {
    let key = LookupKey {
        texture_format: format!("{format:?}"),
        is_array,
    };

    if let Some(pipeline) =
        MIPMAP_PIPELINE.with(|pipeline_cell| pipeline_cell.borrow().get(&key).cloned())
    {
        return Ok(pipeline);
    }

    let shader_source = mipmap_shader_source(format.clone(), is_array)?;
    let shader_module = gpu
        .compile_shader(&ShaderModuleDescriptor::new(&shader_source, Some("Mipmap Shader")).into());

    shader_module.validate_shader().await?;

    let compute = ProgrammableStage::new(&shader_module, None);

    let view_dimension = if is_array {
        TextureViewDimension::N2dArray
    } else {
        TextureViewDimension::N2d
    };

    let bind_group_layout = gpu.create_bind_group_layout(
        &BindGroupLayoutDescriptor::new(Some("Mipmap Bind Group Layout"))
            .with_entries(vec![
                BindGroupLayoutEntry::new(
                    0,
                    BindGroupLayoutResource::Texture(
                        TextureBindingLayout::new()
                            .with_sample_type(TextureSampleType::Float)
                            .with_view_dimension(view_dimension)
                            .with_multisampled(false),
                    ),
                )
                .with_visibility_compute(),
                BindGroupLayoutEntry::new(
                    1,
                    BindGroupLayoutResource::Sampler(
                        SamplerBindingLayout::new()
                            .with_binding_type(SamplerBindingType::Filtering),
                    ),
                )
                .with_visibility_compute(),
                BindGroupLayoutEntry::new(
                    2,
                    BindGroupLayoutResource::StorageTexture(
                        StorageTextureBindingLayout::new(format)
                            .with_view_dimension(view_dimension)
                            .with_access(StorageTextureAccess::WriteOnly),
                    ),
                )
                .with_visibility_compute(),
            ])
            .into(),
    )?;

    let layout = gpu.create_pipeline_layout(
        &PipelineLayoutDescriptor::new(
            Some("Mipmap Pipeline Layout"),
            vec![bind_group_layout.clone()],
        )
        .into(),
    );
    let layout = PipelineLayoutKind::Custom(&layout);

    let pipeline_descriptor =
        ComputePipelineDescriptor::new(compute, layout.clone(), Some("Mipmap Pipeline"));

    // UGH - move the whole thing out of the closure... let async infect everything... look at earlier async comment
    let pipeline = gpu
        .create_compute_pipeline(&pipeline_descriptor.into())
        .await?;

    MIPMAP_PIPELINE.with(|pipeline_cell| {
        let pipeline = MipmapPipeline {
            compute_pipeline: pipeline,
            bind_group_layout,
        };
        pipeline_cell.borrow_mut().insert(key, pipeline.clone());
        Ok(pipeline)
    })
}

pub fn calculate_mipmap_levels(width: u32, height: u32) -> u32 {
    ((width.max(height) as f32).log2().floor() as u32) + 1
}
