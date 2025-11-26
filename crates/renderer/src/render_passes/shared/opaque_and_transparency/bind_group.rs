use awsm_renderer_core::bind_groups::{
    BindGroupLayoutResource, SamplerBindingLayout, SamplerBindingType, TextureBindingLayout,
};
use awsm_renderer_core::error::AwsmCoreError;
use awsm_renderer_core::texture::{TextureSampleType, TextureViewDimension};
use indexmap::IndexSet;

use crate::bind_group_layout::{BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry};
use crate::error::Result;
use crate::{
    bind_group_layout::BindGroupLayoutKey, render_passes::RenderPassInitContext,
    textures::SamplerKey,
};

pub struct TexturePoolDeps {
    pub texture_bind_group_layout_key: BindGroupLayoutKey,
    pub sampler_bind_group_layout_key: BindGroupLayoutKey,
    pub texture_arrays_len: u32,
    pub texture_sampler_keys: IndexSet<SamplerKey>,
}

impl TexturePoolDeps {
    pub fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        // textures
        let device_limits = ctx.gpu.device.limits();
        let texture_arrays_len = ctx.textures.pool.arrays_len();

        let mut texture_entries = Vec::new();

        if texture_arrays_len > device_limits.max_sampled_textures_per_shader_stage() as usize {
            return Err(AwsmCoreError::TexturePoolTooManyArrays {
                total_arrays: texture_arrays_len as u32,
                max_arrays: device_limits.max_sampled_textures_per_shader_stage(),
            }
            .into());
        }

        for i in 0..texture_arrays_len {
            texture_entries.push(BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(
                    TextureBindingLayout::new()
                        .with_view_dimension(TextureViewDimension::N2dArray)
                        .with_sample_type(TextureSampleType::Float),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            });

            let layer_count = ctx
                .textures
                .pool
                .array_by_index(i)
                .map(|arr| arr.images.len())
                .unwrap_or_default();

            if layer_count > device_limits.max_texture_array_layers() as usize {
                return Err(AwsmCoreError::TexturePoolTooManyLayers {
                    array_index: i as u32,
                    total_layers: layer_count as u32,
                    max_layers: device_limits.max_texture_array_layers(),
                }
                .into());
            }
        }

        let texture_bind_group_layout_key = ctx.bind_group_layouts.get_key(
            &ctx.gpu,
            BindGroupLayoutCacheKey {
                entries: texture_entries,
            },
        )?;

        // samplers
        let mut texture_sampler_keys = ctx.textures.pool_sampler_set.clone();

        if texture_sampler_keys.len() > device_limits.max_samplers_per_shader_stage() as usize {
            return Err(AwsmCoreError::TexturePoolTooManySamplers {
                total_samplers: texture_sampler_keys.len() as u32,
                max_samplers: device_limits.max_samplers_per_shader_stage(),
            }
            .into());
        }

        let mut sampler_entries = Vec::new();

        for _ in 0..texture_sampler_keys.len() {
            sampler_entries.push(BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Sampler(
                    SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            });
        }

        let sampler_bind_group_layout_key = ctx.bind_group_layouts.get_key(
            &ctx.gpu,
            BindGroupLayoutCacheKey {
                entries: sampler_entries,
            },
        )?;

        Ok(Self {
            texture_arrays_len: texture_arrays_len as u32,
            texture_bind_group_layout_key,
            texture_sampler_keys,
            sampler_bind_group_layout_key,
        })
    }
}
