use std::{collections::HashMap, sync::LazyLock};

use awsm_renderer_core::{
    bind_groups::{
        BindGroupLayoutResource, SamplerBindingLayout, SamplerBindingType, TextureBindingLayout,
    },
    buffers::{BufferDescriptor, BufferUsage},
    renderer::AwsmRendererWebGpu,
    texture::{TextureSampleType, TextureViewDimension},
};

use super::{AwsmMaterialError, Result};
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

static BUFFER_USAGE: LazyLock<BufferUsage> =
    LazyLock::new(|| BufferUsage::new().with_uniform().with_copy_dst());

pub struct PbrMaterials {
    uniform_buffer: DynamicUniformBuffer<MaterialKey>,
    uniform_buffer_gpu_dirty: bool,
    pub(crate) gpu_buffer: web_sys::GpuBuffer,
}

impl PbrMaterials {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let gpu_buffer = gpu.create_buffer(
            &BufferDescriptor::new(
                Some("Pbr Materials"),
                PbrMaterial::INITIAL_ELEMENTS * PbrMaterial::UNIFORM_BUFFER_BYTE_ALIGNMENT,
                *BUFFER_USAGE,
            )
            .into(),
        )?;

        Ok(Self {
            uniform_buffer: DynamicUniformBuffer::new(
                PbrMaterial::INITIAL_ELEMENTS,
                PbrMaterial::BYTE_SIZE,
                PbrMaterial::UNIFORM_BUFFER_BYTE_ALIGNMENT,
                Some("PbrUniformBuffer".to_string()),
            ),
            uniform_buffer_gpu_dirty: false,
            gpu_buffer,
        })
    }

    pub fn buffer_offset(&self, key: MaterialKey) -> Option<usize> {
        self.uniform_buffer.offset(key)
    }

    pub fn update(&mut self, key: MaterialKey, pbr_material: &mut PbrMaterial) {
        self.uniform_buffer.update_with(key, |offset, data| {
            pbr_material.uniform_buffer_offset = Some(offset);
            let values = pbr_material.uniform_buffer_data();
            data[..values.len()].copy_from_slice(&values);
        });

        self.uniform_buffer_gpu_dirty = true;
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.uniform_buffer_gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "PBR Uniform Buffer GPU write").entered())
            } else {
                None
            };

            if let Some(new_size) = self.uniform_buffer.take_gpu_needs_resize() {
                self.gpu_buffer = gpu.create_buffer(
                    &BufferDescriptor::new(Some("Pbr Material"), new_size, *BUFFER_USAGE).into(),
                )?;

                bind_groups.mark_create(BindGroupCreate::PbrMaterialUniformResize);
            }

            gpu.write_buffer(
                &self.gpu_buffer,
                None,
                self.uniform_buffer.raw_slice(),
                None,
                None,
            )?;

            self.uniform_buffer_gpu_dirty = false;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub uniform_buffer_offset: Option<usize>,
    pub base_color_factor: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
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
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
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
