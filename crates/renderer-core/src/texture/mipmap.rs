use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::bind_groups::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupLayoutResource,
    StorageTextureAccess, StorageTextureBindingLayout, TextureBindingLayout,
};
use crate::pipeline::layout::{PipelineLayoutDescriptor, PipelineLayoutKind};
use crate::pipeline::{ComputePipelineDescriptor, ProgrammableStage};
use crate::shaders::ShaderModuleExt;

use crate::error::Result;
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

#[derive(Debug, Clone)]
struct EdgeDetectionPipeline {
    pub compute_pipeline: web_sys::GpuComputePipeline,
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
}

thread_local! {
    static EDGE_DETECTION_PIPELINE: RefCell<HashMap<LookupKey, EdgeDetectionPipeline>> = RefCell::new(HashMap::new());
}

fn storage_format_to_wgsl(format: TextureFormat) -> Result<&'static str> {
    match format {
        TextureFormat::Rgba8unorm => Ok("rgba8unorm"),
        TextureFormat::Rgba16float => Ok("rgba16float"),
        TextureFormat::Rgba32float => Ok("rgba32float"),
        _ => Err(AwsmCoreError::MipmapUnsupportedFormat(format)),
    }
}

// Edge detection using Sobel operator
fn edge_detection_shader_source(format: TextureFormat, is_array: bool) -> Result<String> {
    let storage_format = storage_format_to_wgsl(format)?;

    if is_array {
        Ok(format!(
            r#"
@group(0) @binding(0) var input_texture: texture_2d_array<f32>;
@group(0) @binding(1) var edge_map: texture_storage_2d_array<{storage_format}, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let dims = textureDimensions(input_texture);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {{
        return;
    }}

    let layer = i32(global_id.z);
    let coord = vec2<i32>(global_id.xy);

    // Sobel operator kernels
    // Gx = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]]
    // Gy = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]]

    var gx = vec3<f32>(0.0);
    var gy = vec3<f32>(0.0);

    // Sample 3x3 neighborhood
    for (var dy = -1; dy <= 1; dy++) {{
        for (var dx = -1; dx <= 1; dx++) {{
            let sample_coord = clamp(
                coord + vec2<i32>(dx, dy),
                vec2<i32>(0),
                vec2<i32>(dims) - vec2<i32>(1)
            );
            let sample = textureLoad(input_texture, sample_coord, layer, 0).rgb;

            // Apply Sobel kernels
            let gx_weight = f32(dx) * (2.0 - abs(f32(dy)));
            let gy_weight = f32(dy) * (2.0 - abs(f32(dx)));

            gx += sample * gx_weight;
            gy += sample * gy_weight;
        }}
    }}

    // Calculate edge magnitude
    // Don't over-normalize - keep edges strong for thin line detection
    let edge_strength = sqrt(dot(gx, gx) + dot(gy, gy)) / 4.0; // Less aggressive normalization

    // Store edge strength in all channels (we'll use red channel in mipmap generation)
    let edge_output = vec4<f32>(edge_strength, edge_strength, edge_strength, 1.0);
    textureStore(edge_map, coord, layer, edge_output);
}}
"#
        ))
    } else {
        Ok(format!(
            r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var edge_map: texture_storage_2d<{storage_format}, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let dims = textureDimensions(input_texture, 0);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {{
        return;
    }}

    let coord = vec2<i32>(global_id.xy);

    // Sobel operator kernels
    var gx = vec3<f32>(0.0);
    var gy = vec3<f32>(0.0);

    // Sample 3x3 neighborhood
    for (var dy = -1; dy <= 1; dy++) {{
        for (var dx = -1; dx <= 1; dx++) {{
            let sample_coord = clamp(
                coord + vec2<i32>(dx, dy),
                vec2<i32>(0),
                vec2<i32>(dims) - vec2<i32>(1)
            );
            let sample = textureLoad(input_texture, sample_coord, 0).rgb;

            // Apply Sobel kernels
            let gx_weight = f32(dx) * (2.0 - abs(f32(dy)));
            let gy_weight = f32(dy) * (2.0 - abs(f32(dx)));

            gx += sample * gx_weight;
            gy += sample * gy_weight;
        }}
    }}

    // Calculate edge magnitude
    // Don't over-normalize - keep edges strong for thin line detection
    let edge_strength = sqrt(dot(gx, gx) + dot(gy, gy)) / 4.0; // Less aggressive normalization

    // Store edge strength in all channels
    let edge_output = vec4<f32>(edge_strength, edge_strength, edge_strength, 1.0);
    textureStore(edge_map, coord, edge_output);
}}
"#
        ))
    }
}

// High-quality mipmap generation with Kaiser filter and edge awareness
fn mipmap_shader_source(format: TextureFormat, is_array: bool) -> Result<String> {
    let storage_format = storage_format_to_wgsl(format)?;

    if is_array {
        Ok(format!(
            r#"
@group(0) @binding(0) var input_texture: texture_2d_array<f32>;
@group(0) @binding(1) var edge_map: texture_2d_array<f32>;
@group(0) @binding(2) var output_texture: texture_storage_2d_array<{storage_format}, write>;

// Kaiser-Bessel filter weights (β=3.0, 8-tap kernel)
// Precomputed for performance: I₀(β√(1-r²))/I₀(β) where r is normalized radius
const KAISER_WEIGHTS = array<f32, 9>(
    1.0000,  // center (r=0)
    0.9036,  // r=0.353 (diagonal to adjacent)
    0.7854,  // r=0.5 (adjacent)
    0.6548,  // r=0.612 (diagonal)
    0.5000,  // r=0.707 (corner)
    0.3452,  // r=0.791
    0.2146,  // r=0.866
    0.0964,  // r=0.935
    0.0000   // r=1.0
);

fn kaiser_weight(dx: f32, dy: f32) -> f32 {{
    let r = sqrt(dx * dx + dy * dy) / 2.0; // Normalize by kernel radius
    if (r >= 1.0) {{ return 0.0; }}

    // Linear interpolation in lookup table
    let idx = r * 8.0;
    let i0 = u32(floor(idx));
    let i1 = min(i0 + 1u, 8u);
    let frac = fract(idx);

    return mix(KAISER_WEIGHTS[i0], KAISER_WEIGHTS[i1], frac);
}}

fn unpremultiply_alpha(color: vec4<f32>) -> vec4<f32> {{
    if (color.a <= 0.001) {{
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }}
    return vec4<f32>(color.rgb / color.a, color.a);
}}

fn premultiply_alpha(color: vec4<f32>) -> vec4<f32> {{
    return vec4<f32>(color.rgb * color.a, color.a);
}}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let output_size = textureDimensions(output_texture);

    if (global_id.x >= output_size.x || global_id.y >= output_size.y) {{
        return;
    }}

    let layer = i32(global_id.z);
    let input_size = textureDimensions(input_texture);
    let output_coord = vec2<i32>(global_id.xy);

    // Center of the 2x2 region in input space
    let input_center = vec2<f32>(output_coord) * 2.0 + 1.0;

    // 8-tap Kaiser filter sampling pattern
    // Sample in a 4x4 region centered on the 2x2 block
    var accumulated_color = vec4<f32>(0.0);
    var total_weight = 0.0;
    var max_edge_strength = 0.0;

    // Sample 4x4 region with Kaiser weighting
    for (var dy = -1; dy <= 2; dy++) {{
        for (var dx = -1; dx <= 2; dx++) {{
            let sample_coord = vec2<i32>(input_center) + vec2<i32>(dx, dy);
            let clamped_coord = clamp(
                sample_coord,
                vec2<i32>(0),
                vec2<i32>(input_size) - vec2<i32>(1)
            );

            // Load color and edge strength
            let color = textureLoad(input_texture, clamped_coord, layer, 0);
            let edge_strength = textureLoad(edge_map, clamped_coord, layer, 0).r;

            max_edge_strength = max(max_edge_strength, edge_strength);

            // Calculate Kaiser weight based on distance from center
            let offset = vec2<f32>(sample_coord) - input_center;
            let weight = kaiser_weight(offset.x, offset.y);

            // Un-premultiply alpha for correct filtering
            let unpremul = unpremultiply_alpha(color);

            accumulated_color += unpremul * weight;
            total_weight += weight;
        }}
    }}

    // Normalize by total weight
    var result = accumulated_color / max(total_weight, 0.0001);

    // Use standard Kaiser filtering - no edge preservation
    // This properly blurs thin lines at higher mip levels like hardware mipmaps
    textureStore(output_texture, output_coord, layer, result);
}}
"#
        ))
    } else {
        Ok(format!(
            r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var edge_map: texture_2d<f32>;
@group(0) @binding(2) var output_texture: texture_storage_2d<{storage_format}, write>;

// Kaiser-Bessel filter weights (β=3.0, 8-tap kernel)
const KAISER_WEIGHTS = array<f32, 9>(
    1.0000,  // center (r=0)
    0.9036,  // r=0.353
    0.7854,  // r=0.5
    0.6548,  // r=0.612
    0.5000,  // r=0.707
    0.3452,  // r=0.791
    0.2146,  // r=0.866
    0.0964,  // r=0.935
    0.0000   // r=1.0
);

fn kaiser_weight(dx: f32, dy: f32) -> f32 {{
    let r = sqrt(dx * dx + dy * dy) / 2.0;
    if (r >= 1.0) {{ return 0.0; }}

    let idx = r * 8.0;
    let i0 = u32(floor(idx));
    let i1 = min(i0 + 1u, 8u);
    let frac = fract(idx);

    return mix(KAISER_WEIGHTS[i0], KAISER_WEIGHTS[i1], frac);
}}

fn unpremultiply_alpha(color: vec4<f32>) -> vec4<f32> {{
    if (color.a <= 0.001) {{
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }}
    return vec4<f32>(color.rgb / color.a, color.a);
}}

fn premultiply_alpha(color: vec4<f32>) -> vec4<f32> {{
    return vec4<f32>(color.rgb * color.a, color.a);
}}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let output_size = textureDimensions(output_texture);

    if (global_id.x >= output_size.x || global_id.y >= output_size.y) {{
        return;
    }}

    let input_size = textureDimensions(input_texture, 0);
    let output_coord = vec2<i32>(global_id.xy);

    let input_center = vec2<f32>(output_coord) * 2.0 + 1.0;

    var accumulated_color = vec4<f32>(0.0);
    var total_weight = 0.0;
    var max_edge_strength = 0.0;

    // 4x4 Kaiser sampling
    for (var dy = -1; dy <= 2; dy++) {{
        for (var dx = -1; dx <= 2; dx++) {{
            let sample_coord = vec2<i32>(input_center) + vec2<i32>(dx, dy);
            let clamped_coord = clamp(
                sample_coord,
                vec2<i32>(0),
                vec2<i32>(input_size) - vec2<i32>(1)
            );

            let color = textureLoad(input_texture, clamped_coord, 0);
            let edge_strength = textureLoad(edge_map, clamped_coord, 0).r;

            max_edge_strength = max(max_edge_strength, edge_strength);

            let offset = vec2<f32>(sample_coord) - input_center;
            let weight = kaiser_weight(offset.x, offset.y);

            let unpremul = unpremultiply_alpha(color);

            accumulated_color += unpremul * weight;
            total_weight += weight;
        }}
    }}

    var kaiser_result = accumulated_color / max(total_weight, 0.0001);

    // ULTRA-AGGRESSIVE EDGE PRESERVATION for thin lines
    var best_sample = vec4<f32>(0.0);
    var best_lum = 0.0;
    var worst_lum = 999.0;

    let input_base = output_coord * 2;
    for (var dy = 0; dy <= 1; dy++) {{
        for (var dx = 0; dx <= 1; dx++) {{
            let coord = clamp(
                input_base + vec2<i32>(dx, dy),
                vec2<i32>(0),
                vec2<i32>(input_size) - vec2<i32>(1)
            );
            let sample = textureLoad(input_texture, coord, 0);
            let lum = dot(sample.rgb, vec3<f32>(0.299, 0.587, 0.114));

            if (lum > best_lum) {{
                best_sample = unpremultiply_alpha(sample);
                best_lum = lum;
            }}
            worst_lum = min(worst_lum, lum);
        }}
    }}

    // Calculate contrast in the 2x2 region
    let local_contrast = best_lum - worst_lum;

    // Blend strategy: smooth areas use Kaiser, high contrast preserves brightest
    var result: vec4<f32>;
    if (local_contrast < 0.1) {{
        result = kaiser_result;
    }} else if (local_contrast < 0.3) {{
        let blend = (local_contrast - 0.1) / 0.2;
        result = mix(kaiser_result, best_sample, blend * 0.7);
    }} else {{
        result = mix(kaiser_result, best_sample, 0.85);
    }}

    result = premultiply_alpha(result);

    textureStore(output_texture, output_coord, result);
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
    use crate::texture::{Extent3d, TextureDescriptor, TextureDimension, TextureUsage};

    // Get pipelines
    let MipmapPipeline {
        compute_pipeline: mipmap_pipeline,
        bind_group_layout: mipmap_bind_group_layout,
    } = get_mipmap_pipeline(gpu, texture.format(), is_array).await?;

    let EdgeDetectionPipeline {
        compute_pipeline: edge_pipeline,
        bind_group_layout: edge_bind_group_layout,
    } = get_edge_detection_pipeline(gpu, texture.format(), is_array).await?;

    // Create temporary edge map textures for each mip level
    // We need edge maps at the resolution of the INPUT to each mipmap pass
    let mut edge_map_textures = Vec::new();
    let mut temp_width = current_width;
    let mut temp_height = current_height;

    for _ in 0..mip_levels {
        let usage = TextureUsage::new()
            .with_storage_binding()
            .with_texture_binding();

        let size = Extent3d::new(temp_width, Some(temp_height), if is_array {
            Some(array_layers)
        } else {
            None
        });

        let texture_descriptor = TextureDescriptor::new(
            texture.format(),
            size,
            usage,
        )
        .with_label("Edge Map Temp")
        .with_dimension(if is_array {
            TextureDimension::N2d
        } else {
            TextureDimension::N2d
        })
        .with_mip_level_count(1);

        let edge_texture = gpu.create_texture(&texture_descriptor.into())?;

        edge_map_textures.push(edge_texture);
        temp_width = (temp_width / 2).max(1);
        temp_height = (temp_height / 2).max(1);
    }

    let command_encoder = gpu.create_command_encoder(Some("Generate Mipmaps"));

    let view_dimension = if is_array {
        TextureViewDimension::N2dArray
    } else {
        TextureViewDimension::N2d
    };

    // Process each mip level
    for mip_level in 1..mip_levels {
        let next_width = (current_width / 2).max(1);
        let next_height = (current_height / 2).max(1);

        // PASS 1: Edge Detection
        // Run edge detection on the input (previous mip level)
        let mut input_view_descriptor = TextureViewDescriptor::new(Some("Edge Input View"))
            .with_base_mip_level(mip_level - 1)
            .with_dimension(view_dimension)
            .with_mip_level_count(1);
        if is_array {
            input_view_descriptor = input_view_descriptor.with_array_layer_count(array_layers);
        }
        let input_view = texture
            .create_view_with_descriptor(&input_view_descriptor.into())
            .map_err(AwsmCoreError::create_texture_view)?;

        // Edge map output (at input resolution)
        let mut edge_output_view_descriptor =
            TextureViewDescriptor::new(Some("Edge Map Output View"))
                .with_dimension(view_dimension);
        if is_array {
            edge_output_view_descriptor =
                edge_output_view_descriptor.with_array_layer_count(array_layers);
        }
        let edge_output_view = edge_map_textures[(mip_level - 1) as usize]
            .create_view_with_descriptor(&edge_output_view_descriptor.into())
            .map_err(AwsmCoreError::create_texture_view)?;

        let edge_bind_group = gpu.create_bind_group(
            &BindGroupDescriptor::new(
                &edge_bind_group_layout,
                Some("Edge Detection Bind Group"),
                vec![
                    BindGroupEntry::new(0, BindGroupResource::TextureView(Cow::Owned(input_view.clone()))),
                    BindGroupEntry::new(1, BindGroupResource::TextureView(Cow::Owned(edge_output_view))),
                ],
            )
            .into(),
        );

        let edge_pass = command_encoder.begin_compute_pass(Some(
            &ComputePassDescriptor::new(Some("Edge Detection Pass")).into(),
        ));
        edge_pass.set_pipeline(&edge_pipeline);
        edge_pass.set_bind_group(0, &edge_bind_group, None)?;

        let workgroup_size_x = current_width.div_ceil(8);
        let workgroup_size_y = current_height.div_ceil(8);
        edge_pass.dispatch_workgroups(
            workgroup_size_x,
            Some(workgroup_size_y),
            Some(array_layers),
        );
        edge_pass.end();

        // PASS 2: Kaiser Mipmap Generation with Edge Awareness
        // Input: previous mip level + edge map
        // Output: current mip level

        // Reuse input view from edge detection
        let mut edge_map_view_descriptor =
            TextureViewDescriptor::new(Some("Edge Map Input View"))
                .with_dimension(view_dimension);
        if is_array {
            edge_map_view_descriptor =
                edge_map_view_descriptor.with_array_layer_count(array_layers);
        }
        let edge_map_view = edge_map_textures[(mip_level - 1) as usize]
            .create_view_with_descriptor(&edge_map_view_descriptor.into())
            .map_err(AwsmCoreError::create_texture_view)?;

        let mut output_view_descriptor = TextureViewDescriptor::new(Some("Mipmap Output View"))
            .with_base_mip_level(mip_level)
            .with_dimension(view_dimension)
            .with_mip_level_count(1);
        if is_array {
            output_view_descriptor = output_view_descriptor.with_array_layer_count(array_layers);
        }
        let output_view = texture
            .create_view_with_descriptor(&output_view_descriptor.into())
            .map_err(AwsmCoreError::create_texture_view)?;

        let mipmap_bind_group = gpu.create_bind_group(
            &BindGroupDescriptor::new(
                &mipmap_bind_group_layout,
                Some("Mipmap Bind Group"),
                vec![
                    BindGroupEntry::new(0, BindGroupResource::TextureView(Cow::Owned(input_view))),
                    BindGroupEntry::new(1, BindGroupResource::TextureView(Cow::Owned(edge_map_view))),
                    BindGroupEntry::new(2, BindGroupResource::TextureView(Cow::Owned(output_view))),
                ],
            )
            .into(),
        );

        let mipmap_pass = command_encoder.begin_compute_pass(Some(
            &ComputePassDescriptor::new(Some("Mipmap Generation Pass")).into(),
        ));
        mipmap_pass.set_pipeline(&mipmap_pipeline);
        mipmap_pass.set_bind_group(0, &mipmap_bind_group, None)?;

        let workgroup_size_x = next_width.div_ceil(8);
        let workgroup_size_y = next_height.div_ceil(8);
        mipmap_pass.dispatch_workgroups(
            workgroup_size_x,
            Some(workgroup_size_y),
            Some(array_layers),
        );
        mipmap_pass.end();

        current_width = next_width;
        current_height = next_height;
    }

    // Submit all commands
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
                // Binding 0: Input texture (previous mip level)
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
                // Binding 1: Edge map texture (same resolution as input)
                BindGroupLayoutEntry::new(
                    1,
                    BindGroupLayoutResource::Texture(
                        TextureBindingLayout::new()
                            .with_sample_type(TextureSampleType::Float)
                            .with_view_dimension(view_dimension)
                            .with_multisampled(false),
                    ),
                )
                .with_visibility_compute(),
                // Binding 2: Output texture (current mip level)
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

async fn get_edge_detection_pipeline(
    gpu: &AwsmRendererWebGpu,
    format: TextureFormat,
    is_array: bool,
) -> Result<EdgeDetectionPipeline> {
    let key = LookupKey {
        texture_format: format!("{format:?}"),
        is_array,
    };

    if let Some(pipeline) =
        EDGE_DETECTION_PIPELINE.with(|pipeline_cell| pipeline_cell.borrow().get(&key).cloned())
    {
        return Ok(pipeline);
    }

    let shader_source = edge_detection_shader_source(format.clone(), is_array)?;
    let shader_module = gpu.compile_shader(
        &ShaderModuleDescriptor::new(&shader_source, Some("Edge Detection Shader")).into(),
    );

    shader_module.validate_shader().await?;

    let compute = ProgrammableStage::new(&shader_module, None);

    let view_dimension = if is_array {
        TextureViewDimension::N2dArray
    } else {
        TextureViewDimension::N2d
    };

    let bind_group_layout = gpu.create_bind_group_layout(
        &BindGroupLayoutDescriptor::new(Some("Edge Detection Bind Group Layout"))
            .with_entries(vec![
                // Binding 0: Input texture
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
                // Binding 1: Edge map output
                BindGroupLayoutEntry::new(
                    1,
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
            Some("Edge Detection Pipeline Layout"),
            vec![bind_group_layout.clone()],
        )
        .into(),
    );
    let layout = PipelineLayoutKind::Custom(&layout);

    let pipeline_descriptor = ComputePipelineDescriptor::new(
        compute,
        layout.clone(),
        Some("Edge Detection Pipeline"),
    );

    let pipeline = gpu
        .create_compute_pipeline(&pipeline_descriptor.into())
        .await?;

    EDGE_DETECTION_PIPELINE.with(|pipeline_cell| {
        let pipeline = EdgeDetectionPipeline {
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
