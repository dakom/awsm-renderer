use std::{collections::HashMap, sync::LazyLock};

use awsm_renderer_core::{
    bind_groups::{
        BindGroupLayoutResource, SamplerBindingLayout, SamplerBindingType, TextureBindingLayout,
    },
    buffers::{BufferDescriptor, BufferUsage},
    renderer::AwsmRendererWebGpu,
    sampler::AddressMode,
    texture::{mega_texture::MegaTextureEntryInfo, TextureSampleType, TextureViewDimension},
};

use crate::materials::{AwsmMaterialError, Result};
use crate::{
    bind_group_layout::{
        BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry, BindGroupLayoutKey,
    },
    bind_groups::{BindGroupCreate, BindGroups},
    buffer::dynamic_uniform::DynamicUniformBuffer,
    materials::{MaterialAlphaMode, MaterialKey},
    textures::{SamplerKey, TextureKey, Textures},
    AwsmRenderer, AwsmRendererLogging,
};

#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub base_color_tex: Option<TextureKey>,
    pub base_color_sampler: Option<SamplerKey>,
    pub base_color_uv_index: Option<u32>,
    pub base_color_factor: [f32; 4],
    pub metallic_roughness_tex: Option<TextureKey>,
    pub metallic_roughness_sampler: Option<SamplerKey>,
    pub metallic_roughness_uv_index: Option<u32>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_tex: Option<TextureKey>,
    pub normal_sampler: Option<SamplerKey>,
    pub normal_uv_index: Option<u32>,
    pub normal_scale: f32,
    pub occlusion_tex: Option<TextureKey>,
    pub occlusion_sampler: Option<SamplerKey>,
    pub occlusion_uv_index: Option<u32>,
    pub occlusion_strength: f32,
    pub emissive_tex: Option<TextureKey>,
    pub emissive_sampler: Option<SamplerKey>,
    pub emissive_uv_index: Option<u32>,
    pub emissive_factor: [f32; 3],
    pub emissive_strength: f32,
    pub vertex_color_info: Option<VertexColorInfo>,
    // these come from initial settings which affects bind group, mesh pipeline etc.
    // so the only way to change them is to create a new material
    alpha_mode: MaterialAlphaMode,
    double_sided: bool,
}

#[derive(Clone, Debug)]
pub struct VertexColorInfo {
    pub set_index: u32,
}

impl PbrMaterial {
    pub const INITIAL_ELEMENTS: usize = 32; // 32 elements is a good starting point
                                            // NOTE: keep this in sync with `PbrMaterialRaw` in WGSL. Each texture packs 56 bytes
                                            // (including sampler + address mode + padding + UV transforms) so 5 textures + 60 byte header + padding = 348.
    pub const BYTE_SIZE: usize = 348; // must be under Materials::MAX_SIZE

    pub const BITMASK_BASE_COLOR: u32 = 1;
    pub const BITMASK_METALIC_ROUGHNESS: u32 = 1 << 1;
    pub const BITMASK_NORMAL: u32 = 1 << 2;
    pub const BITMASK_OCCLUSION: u32 = 1 << 3;
    pub const BITMASK_EMISSIVE: u32 = 1 << 4;
    pub const BITMASK_VERTEX_COLOR: u32 = 1 << 5;

    pub fn new(alpha_mode: MaterialAlphaMode, double_sided: bool) -> Self {
        Self {
            alpha_mode,
            double_sided,
            base_color_tex: None,
            base_color_sampler: None,
            base_color_uv_index: None,
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic_roughness_tex: None,
            metallic_roughness_sampler: None,
            metallic_roughness_uv_index: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            normal_tex: None,
            normal_sampler: None,
            normal_uv_index: None,
            normal_scale: 1.0,
            occlusion_tex: None,
            occlusion_sampler: None,
            occlusion_uv_index: None,
            occlusion_strength: 1.0,
            emissive_tex: None,
            emissive_sampler: None,
            emissive_uv_index: None,
            emissive_factor: [0.0, 0.0, 0.0],
            emissive_strength: 1.0,
            vertex_color_info: None,
        }
    }

    pub fn alpha_mode(&self) -> &MaterialAlphaMode {
        &self.alpha_mode
    }

    pub fn double_sided(&self) -> bool {
        self.double_sided
    }

    pub fn set_alpha_cutoff(&mut self, cutoff: f32) -> Result<()> {
        if let MaterialAlphaMode::Mask { .. } = self.alpha_mode {
            self.alpha_mode = MaterialAlphaMode::Mask { cutoff };
            Ok(())
        } else {
            Err(AwsmMaterialError::InvalidAlphaModeForCutoff(
                self.alpha_mode,
            ))
        }
    }

    pub fn alpha_cutoff(&self) -> Option<f32> {
        match self.alpha_mode {
            MaterialAlphaMode::Mask { cutoff } => Some(cutoff),
            _ => None,
        }
    }

    pub fn has_alpha_blend(&self) -> bool {
        matches!(self.alpha_mode, MaterialAlphaMode::Blend)
    }

    pub fn uniform_buffer_data(&self, textures: &Textures) -> Result<[u8; Self::BYTE_SIZE]> {
        let mut data = [0u8; Self::BYTE_SIZE];
        let mut offset = 0;

        enum Value<'a> {
            F32(f32),
            U32(u32),
            Texture {
                entry_info: &'a MegaTextureEntryInfo<TextureKey>,
                uv_index: u32,
                sampler_index: u32,
                address_mode_u: u32,
                address_mode_v: u32,
                padding: u32,
                atlas_size: u32,
            },
            SkipTexture,
        }

        impl From<f32> for Value<'_> {
            fn from(value: f32) -> Self {
                Value::F32(value)
            }
        }
        impl From<u32> for Value<'_> {
            fn from(value: u32) -> Self {
                Value::U32(value)
            }
        }

        impl<'a>
            From<(
                &'a MegaTextureEntryInfo<TextureKey>,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
            )> for Value<'a>
        {
            fn from(
                value: (
                    &'a MegaTextureEntryInfo<TextureKey>,
                    u32,
                    u32,
                    u32,
                    u32,
                    u32,
                    u32,
                ),
            ) -> Self {
                Value::Texture {
                    entry_info: value.0,
                    uv_index: value.1,
                    sampler_index: value.2,
                    address_mode_u: value.3,
                    address_mode_v: value.4,
                    padding: value.5,
                    atlas_size: value.6,
                }
            }
        }

        let mut write = |value: Value| {
            fn write_inner(data: &mut [u8], value: Value, mut offset: usize) -> usize {
                match value {
                    Value::F32(value) => {
                        let bytes = value.to_ne_bytes();
                        data[offset..offset + 4].copy_from_slice(&bytes);
                        offset += 4;
                    }
                    Value::U32(value) => {
                        let bytes = value.to_ne_bytes();
                        data[offset..offset + 4].copy_from_slice(&bytes);
                        offset += 4;
                    }
                    Value::Texture {
                        entry_info,
                        uv_index,
                        sampler_index,
                        address_mode_u,
                        address_mode_v,
                        padding,
                        atlas_size,
                    } => {
                        offset = write_inner(data, entry_info.pixel_offset[0].into(), offset);
                        offset = write_inner(data, entry_info.pixel_offset[1].into(), offset);
                        offset = write_inner(data, entry_info.size[0].into(), offset);
                        offset = write_inner(data, entry_info.size[1].into(), offset);

                        let packed_index_1 = (entry_info.index.atlas as u32)
                            | ((entry_info.index.layer as u32) << 16);
                        let packed_index_2 = (entry_info.index.entry as u32) | (uv_index << 16);

                        offset = write_inner(data, packed_index_1.into(), offset);
                        offset = write_inner(data, packed_index_2.into(), offset);
                        offset = write_inner(data, sampler_index.into(), offset);
                        offset = write_inner(data, address_mode_u.into(), offset);
                        offset = write_inner(data, address_mode_v.into(), offset);
                        offset = write_inner(data, padding.into(), offset);

                        // Compute UV transform for optimized GPU sampling
                        let atlas_dimensions = atlas_size as f32;
                        let texel_offset_x = entry_info.pixel_offset[0] as f32;
                        let texel_offset_y = entry_info.pixel_offset[1] as f32;
                        let texel_size_x = entry_info.size[0] as f32;
                        let texel_size_y = entry_info.size[1] as f32;

                        let span_x = (texel_size_x - 1.0).max(0.0);
                        let span_y = (texel_size_y - 1.0).max(0.0);

                        // uv_offset = (texel_offset + 0.5) / atlas_dimensions
                        let uv_offset_x = (texel_offset_x + 0.5) / atlas_dimensions;
                        let uv_offset_y = (texel_offset_y + 0.5) / atlas_dimensions;

                        // uv_scale = span / atlas_dimensions
                        let uv_scale_x = span_x / atlas_dimensions;
                        let uv_scale_y = span_y / atlas_dimensions;

                        offset = write_inner(data, uv_offset_x.into(), offset);
                        offset = write_inner(data, uv_offset_y.into(), offset);
                        offset = write_inner(data, uv_scale_x.into(), offset);
                        offset = write_inner(data, uv_scale_y.into(), offset);
                    }
                    Value::SkipTexture => {
                        offset += 56; // 14 * 4 bytes
                    }
                }

                offset
            }

            offset = write_inner(&mut data, value, offset);
        };

        write(self.alpha_mode.variant_as_u32().into());
        write(self.alpha_cutoff().unwrap_or(0.0f32).into());
        write(if self.double_sided {
            1u32.into()
        } else {
            0u32.into()
        });

        write(self.base_color_factor[0].into());
        write(self.base_color_factor[1].into());
        write(self.base_color_factor[2].into());
        write(self.base_color_factor[3].into());

        write(self.metallic_factor.into());
        write(self.roughness_factor.into());
        write(self.normal_scale.into());
        write(self.occlusion_strength.into());

        write(self.emissive_factor[0].into());
        write(self.emissive_factor[1].into());
        write(self.emissive_factor[2].into());

        write(self.emissive_strength.into());

        // Encode the WebGPU address mode so the shader can reproduce clamp/repeat/mirror behaviour
        // after the sampling coordinates are adjusted to the mega texture tile.
        let encode_address_mode = |mode: Option<AddressMode>| -> u32 {
            match mode.unwrap_or(AddressMode::Repeat) {
                AddressMode::ClampToEdge => 0,
                AddressMode::Repeat => 1,
                AddressMode::MirrorRepeat => 2,
                // WebGPU exposes additional vendor-specific variants behind feature flags. If we
                // ever see one, treat it as repeat so rendering keeps working instead of crashing.
                _ => 1,
            }
        };

        let mut bitmask = 0u32;

        let sampler_key_list: Vec<SamplerKey> =
            textures.mega_texture_sampler_set.iter().cloned().collect();

        let texture_padding = textures.mega_texture.padding;
        let atlas_size = textures.mega_texture.texture_size;

        if let Some(tex) = self.base_color_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let sampler_key = self.base_color_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.base_color_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);
            Some((
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                texture_padding,
                atlas_size,
            ))
        }) {
            write(tex.into());
            bitmask |= Self::BITMASK_BASE_COLOR;
        } else {
            write(Value::SkipTexture);
        }

        if let Some(tex) = self.metallic_roughness_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let sampler_key = self.metallic_roughness_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.metallic_roughness_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);

            Some((
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                texture_padding,
                atlas_size,
            ))
        }) {
            write(tex.into());
            bitmask |= Self::BITMASK_METALIC_ROUGHNESS;
        } else {
            write(Value::SkipTexture);
        }

        if let Some(tex) = self.normal_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let sampler_key = self.normal_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.normal_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);
            Some((
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                texture_padding,
                atlas_size,
            ))
        }) {
            write(tex.into());
            bitmask |= Self::BITMASK_NORMAL;
        } else {
            write(Value::SkipTexture);
        }

        if let Some(tex) = self.occlusion_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let sampler_key = self.occlusion_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.occlusion_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);
            Some((
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                texture_padding,
                atlas_size,
            ))
        }) {
            write(tex.into());
            bitmask |= Self::BITMASK_OCCLUSION;
        } else {
            write(Value::SkipTexture);
        }

        if let Some(tex) = self.emissive_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let sampler_key = self.emissive_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.emissive_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);
            Some((
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                texture_padding,
                atlas_size,
            ))
        }) {
            write(tex.into());
            bitmask |= Self::BITMASK_EMISSIVE;
        } else {
            write(Value::SkipTexture);
        }

        if let Some(color_info) = &self.vertex_color_info {
            write(color_info.set_index.into());
            bitmask |= Self::BITMASK_VERTEX_COLOR;
        } else {
            write(0u32.into());
        }

        write(bitmask.into());

        Ok(data)
    }
}
