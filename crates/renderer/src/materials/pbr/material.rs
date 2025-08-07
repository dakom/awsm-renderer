use std::{collections::HashMap, sync::LazyLock};

use awsm_renderer_core::{
    bind_groups::{
        BindGroupLayoutResource, SamplerBindingLayout, SamplerBindingType, TextureBindingLayout,
    },
    buffers::{BufferDescriptor, BufferUsage},
    renderer::AwsmRendererWebGpu,
    texture::{TextureSampleType, TextureViewDimension},
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
    pub uniform_buffer_offset: Option<usize>,
    pub base_color_tex: Option<TextureKey>,
    pub base_color_uv_index: Option<u32>,
    pub base_color_factor: [f32; 4],
    pub metallic_roughness_tex: Option<TextureKey>,
    pub metallic_roughness_uv_index: Option<u32>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_tex: Option<TextureKey>,
    pub normal_uv_index: Option<u32>,
    pub normal_scale: f32,
    pub occlusion_tex: Option<TextureKey>,
    pub occlusion_uv_index: Option<u32>,
    pub occlusion_strength: f32,
    pub emissive_tex: Option<TextureKey>,
    pub emissive_uv_index: Option<u32>,
    pub emissive_factor: [f32; 3],
    // these come from initial settings which affects bind group, mesh pipeline etc.
    // so the only way to change them is to create a new material
    alpha_mode: MaterialAlphaMode,
    double_sided: bool,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            uniform_buffer_offset: None,
            base_color_tex: None,
            base_color_uv_index: None,
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic_roughness_tex: None,
            metallic_roughness_uv_index: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            normal_tex: None,
            normal_uv_index: None,
            normal_scale: 1.0,
            occlusion_tex: None,
            occlusion_uv_index: None,
            occlusion_strength: 1.0,
            emissive_tex: None,
            emissive_uv_index: None,
            emissive_factor: [0.0, 0.0, 0.0],
            alpha_mode: MaterialAlphaMode::Opaque,
            double_sided: false,
        }
    }
}

impl PbrMaterial {
    pub const INITIAL_ELEMENTS: usize = 32; // 32 elements is a good starting point
    pub const UNIFORM_BUFFER_BYTE_ALIGNMENT: usize = 256; // minUniformBufferOffsetAlignment
    pub const BYTE_SIZE: usize = 64;

    pub fn new(alpha_mode: MaterialAlphaMode, double_sided: bool) -> Self {
        Self {
            alpha_mode,
            double_sided,
            ..Default::default()
        }
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

    pub fn uniform_buffer_data(&self) -> [u8; Self::BYTE_SIZE] {
        let mut data = [0u8; Self::BYTE_SIZE];
        let mut offset = 0;

        enum Value {
            F32(f32),
            U32(u32),
        }

        impl From<f32> for Value {
            fn from(value: f32) -> Self {
                Value::F32(value)
            }
        }
        impl From<u32> for Value {
            fn from(value: u32) -> Self {
                Value::U32(value)
            }
        }

        let mut write = |value: Value| match value {
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
        };

        // 16 bytes (4 * 4 byte (32 bit) integers or floats)
        write((self.uniform_buffer_offset.unwrap_or(0) as u32).into());
        write(self.alpha_mode.variant_as_u32().into()); // 4 bytes, offset 4 -> offset 8
        write(self.alpha_cutoff().unwrap_or(0.0f32).into()); // 4 bytes, offset 8 -> offset 12
        write(if self.double_sided {
            1u32.into()
        } else {
            0u32.into()
        }); // 4 bytes, offset 12 -> offset 16

        // 16 bytes (4 * 4 byte (32 bit) floats)
        write(self.base_color_factor[0].into());
        write(self.base_color_factor[1].into());
        write(self.base_color_factor[2].into());
        write(self.base_color_factor[3].into());

        // 16 bytes (4 * 4 byte (32 bit) floats)
        write(self.metallic_factor.into());
        write(self.roughness_factor.into());
        write(self.normal_scale.into());
        write(self.occlusion_strength.into());

        // 12 bytes (3 * 4 byte (32 bit) floats)
        write(self.emissive_factor[0].into());
        write(self.emissive_factor[1].into());
        write(self.emissive_factor[2].into());

        // 4 bytes of padding to align to 64 bytes
        write(0u32.into());

        data
    }
}
