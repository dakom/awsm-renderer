use std::{collections::HashMap, sync::LazyLock};

use awsm_renderer_core::{
    bind_groups::{
        BindGroupLayoutResource, SamplerBindingLayout, SamplerBindingType, TextureBindingLayout,
    },
    buffers::{BufferDescriptor, BufferUsage},
    renderer::AwsmRendererWebGpu,
    sampler::AddressMode,
    texture::{
        mipmap::MipmapTextureKind,
        texture_pool::{TexturePoolArray, TexturePoolEntryInfo},
        TextureSampleType, TextureViewDimension,
    },
};

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
use crate::{
    materials::{AwsmMaterialError, Result},
    textures::TextureTransformKey,
};

#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub base_color_tex: Option<TextureKey>,
    pub base_color_sampler: Option<SamplerKey>,
    pub base_color_texture_transform: Option<TextureTransformKey>,
    pub base_color_uv_index: Option<u32>,
    pub base_color_factor: [f32; 4],
    pub metallic_roughness_tex: Option<TextureKey>,
    pub metallic_roughness_sampler: Option<SamplerKey>,
    pub metallic_roughness_uv_index: Option<u32>,
    pub metallic_roughness_texture_transform: Option<TextureTransformKey>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_tex: Option<TextureKey>,
    pub normal_sampler: Option<SamplerKey>,
    pub normal_uv_index: Option<u32>,
    pub normal_scale: f32,
    pub normal_texture_transform: Option<TextureTransformKey>,
    pub occlusion_tex: Option<TextureKey>,
    pub occlusion_sampler: Option<SamplerKey>,
    pub occlusion_uv_index: Option<u32>,
    pub occlusion_strength: f32,
    pub occlusion_texture_transform: Option<TextureTransformKey>,
    pub emissive_tex: Option<TextureKey>,
    pub emissive_sampler: Option<SamplerKey>,
    pub emissive_uv_index: Option<u32>,
    pub emissive_factor: [f32; 3],
    pub emissive_strength: f32,
    pub emissive_texture_transform: Option<TextureTransformKey>,
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
                                            // NOTE: keep this in sync with `PbrMaterialRaw` in WGSL. Each texture packs 20 bytes
                                            // (compact format) so 5 textures + 60 byte header + 8 bytes = 168.
    pub const BYTE_SIZE: usize = 168; // must be under Materials::MAX_SIZE

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
            base_color_texture_transform: None,
            metallic_roughness_tex: None,
            metallic_roughness_sampler: None,
            metallic_roughness_uv_index: None,
            metallic_factor: 1.0,
            metallic_roughness_texture_transform: None,
            roughness_factor: 1.0,
            normal_tex: None,
            normal_sampler: None,
            normal_uv_index: None,
            normal_scale: 1.0,
            normal_texture_transform: None,
            occlusion_tex: None,
            occlusion_sampler: None,
            occlusion_uv_index: None,
            occlusion_strength: 1.0,
            occlusion_texture_transform: None,
            emissive_tex: None,
            emissive_sampler: None,
            emissive_uv_index: None,
            emissive_factor: [0.0, 0.0, 0.0],
            emissive_strength: 1.0,
            emissive_texture_transform: None,
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

    pub fn alpha_mask(&self) -> Option<f32> {
        match self.alpha_mode {
            MaterialAlphaMode::Mask { cutoff } => Some(cutoff),
            _ => None,
        }
    }

    pub fn uniform_buffer_data(&self, textures: &Textures) -> Result<[u8; Self::BYTE_SIZE]> {
        let mut data = [0u8; Self::BYTE_SIZE];
        let mut offset = 0;

        enum Value<'a> {
            F32(f32),
            U32(u32),
            Texture {
                array: &'a TexturePoolArray<TextureKey>,
                entry_info: &'a TexturePoolEntryInfo<TextureKey>,
                uv_index: u32,
                sampler_index: u32,
                address_mode_u: u32,
                address_mode_v: u32,
                texture_transform_offset: usize,
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
                &'a TexturePoolArray<TextureKey>,
                &'a TexturePoolEntryInfo<TextureKey>,
                u32,
                u32,
                u32,
                u32,
                usize,
            )> for Value<'a>
        {
            fn from(
                value: (
                    &'a TexturePoolArray<TextureKey>,
                    &'a TexturePoolEntryInfo<TextureKey>,
                    u32,
                    u32,
                    u32,
                    u32,
                    usize,
                ),
            ) -> Self {
                Value::Texture {
                    array: value.0,
                    entry_info: value.1,
                    uv_index: value.2,
                    sampler_index: value.3,
                    address_mode_u: value.4,
                    address_mode_v: value.5,
                    texture_transform_offset: value.6,
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
                        array,
                        entry_info,
                        uv_index,
                        sampler_index,
                        address_mode_u,
                        address_mode_v,
                        texture_transform_offset,
                    } => {
                        let packed = pack_texture_info_raw(
                            array,
                            entry_info,
                            uv_index,
                            sampler_index,
                            address_mode_u,
                            address_mode_v,
                            texture_transform_offset,
                        );

                        for word in packed {
                            let bytes = word.to_le_bytes();
                            data[offset..offset + 4].copy_from_slice(&bytes);
                            offset += 4;
                        }
                    }
                    Value::SkipTexture => {
                        offset += 20; // 5 * 4 bytes (packed)
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

        // Encode the WebGPU address mode for mipmap selection.
        // The shader uses this to compute correct UV derivatives when textures wrap/repeat.
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

        let sampler_key_list: Vec<SamplerKey> = textures.pool_sampler_set.iter().cloned().collect();

        if let Some(tex) = self.base_color_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let array = textures.pool.array_by_index(entry_info.array_index)?;
            let sampler_key = self.base_color_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.base_color_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);
            Some((
                array,
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                self.base_color_texture_transform
                    .and_then(|key| textures.get_texture_transform_offset(key))
                    .unwrap_or(textures.texture_transform_identity_offset),
            ))
        }) {
            write(tex.into());
            bitmask |= Self::BITMASK_BASE_COLOR;
        } else {
            write(Value::SkipTexture);
        }

        if let Some(tex) = self.metallic_roughness_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let array = textures.pool.array_by_index(entry_info.array_index)?;
            let sampler_key = self.metallic_roughness_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.metallic_roughness_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);

            Some((
                array,
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                self.metallic_roughness_texture_transform
                    .and_then(|key| textures.get_texture_transform_offset(key))
                    .unwrap_or(textures.texture_transform_identity_offset),
            ))
        }) {
            write(tex.into());
            bitmask |= Self::BITMASK_METALIC_ROUGHNESS;
        } else {
            write(Value::SkipTexture);
        }

        if let Some(tex) = self.normal_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let array = textures.pool.array_by_index(entry_info.array_index)?;
            let sampler_key = self.normal_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.normal_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);
            Some((
                array,
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                self.normal_texture_transform
                    .and_then(|key| textures.get_texture_transform_offset(key))
                    .unwrap_or(textures.texture_transform_identity_offset),
            ))
        }) {
            write(tex.into());
            bitmask |= Self::BITMASK_NORMAL;
        } else {
            write(Value::SkipTexture);
        }

        if let Some(tex) = self.occlusion_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let array = textures.pool.array_by_index(entry_info.array_index)?;
            let sampler_key = self.occlusion_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.occlusion_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);
            Some((
                array,
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                self.occlusion_texture_transform
                    .and_then(|key| textures.get_texture_transform_offset(key))
                    .unwrap_or(textures.texture_transform_identity_offset),
            ))
        }) {
            write(tex.into());
            bitmask |= Self::BITMASK_OCCLUSION;
        } else {
            write(Value::SkipTexture);
        }

        if let Some(tex) = self.emissive_tex.and_then(|texture_key| {
            let entry_info = textures.get_entry(texture_key).ok()?;
            let array = textures.pool.array_by_index(entry_info.array_index)?;
            let sampler_key = self.emissive_sampler?;
            let sampler_index = sampler_key_list.binary_search(&sampler_key).ok()? as u32;
            let uv_index = self.emissive_uv_index?;
            let (address_mode_u, address_mode_v) = textures.sampler_address_modes(sampler_key);
            Some((
                array,
                entry_info,
                uv_index,
                sampler_index,
                encode_address_mode(address_mode_u),
                encode_address_mode(address_mode_v),
                self.emissive_texture_transform
                    .and_then(|key| textures.get_texture_transform_offset(key))
                    .unwrap_or(textures.texture_transform_identity_offset),
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

fn pack_texture_info_raw<ID>(
    array: &TexturePoolArray<ID>,
    entry_info: &TexturePoolEntryInfo<ID>,
    uv_index: u32,
    sampler_index: u32,
    address_mode_u: u32,
    address_mode_v: u32,
    texture_transform_offset: usize,
) -> [u32; 5] {
    // --- size: width (16 bits) + height (16 bits) ---
    let width = array.width;
    let height = array.height;

    debug_assert!(width <= 0xFFFF, "texture width too large for 16 bits");
    debug_assert!(height <= 0xFFFF, "texture height too large for 16 bits");

    let size = (height << 16) | (width & 0xFFFF);

    // --- array_and_layer: array_index (12 bits) + layer_index (20 bits) ---
    let array_index = entry_info.array_index as u32;
    let layer_index = entry_info.layer_index as u32;

    debug_assert!(array_index <= 0xFFF, "array_index too large for 12 bits");
    debug_assert!(layer_index <= 0xFFFFF, "layer_index too large for 20 bits");

    let array_and_layer = (layer_index << 12) | (array_index & 0xFFF);

    // --- uv_and_sampler: uv_set_index (8 bits) + sampler_index (24 bits) ---
    debug_assert!(uv_index <= 0xFF, "uv_index too large for 8 bits");
    debug_assert!(
        sampler_index <= 0xFFFFFF,
        "sampler_index too large for 24 bits"
    );

    let uv_and_sampler = (sampler_index << 8) | (uv_index & 0xFF);

    // --- extra: flags (8) + addr_u (8) + addr_v (8) + padding (8) ---
    // flags:
    //   bit 0: has mipmaps
    let has_mipmaps = array.mipmap;

    let mut flags: u32 = 0;
    if has_mipmaps {
        flags |= 1 << 0;
    }

    debug_assert!(
        address_mode_u <= 0xFF,
        "address_mode_u too large for 8 bits"
    );
    debug_assert!(
        address_mode_v <= 0xFF,
        "address_mode_v too large for 8 bits"
    );

    let extra = (flags & 0xFF) | ((address_mode_u & 0xFF) << 8) | ((address_mode_v & 0xFF) << 16);
    // top 8 bits left as 0 (padding/reserved)

    // --- transform_offset: full 32 bits for byte offset ---
    let transform_offset = texture_transform_offset as u32;

    [
        size,
        array_and_layer,
        uv_and_sampler,
        extra,
        transform_offset,
    ]
}
