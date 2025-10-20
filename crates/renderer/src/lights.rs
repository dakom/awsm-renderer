pub mod ibl;

use std::sync::LazyLock;

use awsm_renderer_core::{
    brdf_lut::generate::BrdfLut,
    buffers::{BufferDescriptor, BufferUsage},
    cubemap::CubemapImage,
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::{BindGroupCreate, BindGroups},
    buffer::dynamic_uniform::DynamicUniformBuffer,
    lights::ibl::Ibl,
    textures::CubemapTextureKey,
    AwsmRenderer, AwsmRendererLogging,
};

static PUNCTUAL_BUFFER_USAGE: LazyLock<BufferUsage> =
    LazyLock::new(|| BufferUsage::new().with_storage().with_copy_dst());

static IBL_UNIFORM_BUFFER_USAGE: LazyLock<BufferUsage> =
    LazyLock::new(|| BufferUsage::new().with_uniform().with_copy_dst());

impl AwsmRenderer {
    pub fn set_brdf_lut(&mut self, brdf_lut: BrdfLut) {
        self.lights.brdf_lut = brdf_lut;
        self.bind_groups.mark_create(BindGroupCreate::BrdfLutCreate);
    }
    pub fn set_ibl(&mut self, ibl: Ibl) {
        self.lights.ibl = ibl;
        self.bind_groups.mark_create(BindGroupCreate::IblCreate);
        self.lights.ibl_uniform_gpu_dirty = true;
    }
}

pub struct Lights {
    pub gpu_punctual_buffer: web_sys::GpuBuffer,
    pub gpu_ibl_buffer: web_sys::GpuBuffer,
    pub ibl: Ibl,
    pub brdf_lut: BrdfLut,
    lights: SlotMap<LightKey, Light>,
    // we use it as a storage buffer, because we need dynamic lengths, but it's a fixed size like a uniform
    punctual_storage_buffer: DynamicUniformBuffer<LightKey>,
    punctual_gpu_dirty: bool,
    ibl_uniform_gpu_dirty: bool,
}

impl Lights {
    pub const INITIAL_ELEMENTS: usize = 8; // 8 lights is a decent baseline
    pub const BYTE_ALIGNMENT: usize = 64; // we aren't using it as a uniform buffer, so storage rules apply
    pub const BYTE_SIZE: usize = 64;
    pub const IBL_UNIFORM_SIZE: usize = 8; // 2 * u32 for mipmap counts

    pub fn new(gpu: &AwsmRendererWebGpu, ibl: Ibl, brdf_lut: BrdfLut) -> Result<Self> {
        let gpu_punctual_buffer = gpu.create_buffer(
            &BufferDescriptor::new(
                Some("Punctual Lights"),
                Self::INITIAL_ELEMENTS * Self::BYTE_ALIGNMENT,
                *PUNCTUAL_BUFFER_USAGE,
            )
            .into(),
        )?;

        let gpu_ibl_buffer = gpu.create_buffer(
            &BufferDescriptor::new(
                Some("IBL Lights"),
                Self::IBL_UNIFORM_SIZE,
                *IBL_UNIFORM_BUFFER_USAGE,
            )
            .into(),
        )?;

        Ok(Lights {
            lights: SlotMap::with_key(),
            punctual_storage_buffer: DynamicUniformBuffer::new(
                Self::INITIAL_ELEMENTS,
                Self::BYTE_SIZE,
                Some(Self::BYTE_ALIGNMENT),
                Some("Lights".to_string()),
            ),
            ibl,
            brdf_lut,
            punctual_gpu_dirty: true,
            ibl_uniform_gpu_dirty: true,
            gpu_punctual_buffer,
            gpu_ibl_buffer,
        })
    }

    pub fn insert(&mut self, light: Light) -> Result<LightKey> {
        let key = self.lights.insert(light.clone());

        self.punctual_storage_buffer
            .update(key, &light.storage_buffer_data());

        self.punctual_gpu_dirty = true;
        Ok(key)
    }

    pub fn remove(&mut self, key: LightKey) {
        self.punctual_storage_buffer.remove(key);
        self.lights.remove(key);
        self.punctual_gpu_dirty = true;
    }

    pub fn update(&mut self, key: LightKey, f: impl FnOnce(&mut Light)) {
        if let Some(light) = self.lights.get_mut(key) {
            f(light);
            self.punctual_storage_buffer
                .update(key, &light.storage_buffer_data());
            self.punctual_gpu_dirty = true;
        }
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.punctual_gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(
                    tracing::span!(
                        tracing::Level::INFO,
                        "Punctual Lights Storage Buffer GPU write"
                    )
                    .entered(),
                )
            } else {
                None
            };

            if let Some(new_size) = self.punctual_storage_buffer.take_gpu_needs_resize() {
                self.gpu_punctual_buffer = gpu.create_buffer(
                    &BufferDescriptor::new(Some("Lights"), new_size, *PUNCTUAL_BUFFER_USAGE).into(),
                )?;

                bind_groups.mark_create(BindGroupCreate::LightsResize);
            }

            gpu.write_buffer(
                &self.gpu_punctual_buffer,
                None,
                self.punctual_storage_buffer.raw_slice(),
                None,
                None,
            )?;

            // for (index, chunk) in self.storage_buffer.raw_slice().chunks_exact(64).enumerate() {
            //     let values = unsafe {
            //         std::slice::from_raw_parts(chunk.as_ptr() as *const f32, 16)
            //     };
            //     tracing::info!("{}: {:?}", index, values);
            // }

            // tracing::info!("n_lights should be {}", self.storage_buffer.raw_slice().len() / (4 * 16));

            self.punctual_gpu_dirty = false;
        }

        if self.ibl_uniform_gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "IBL Uniform Buffer GPU write").entered())
            } else {
                None
            };

            let mut data = vec![0u8; Self::IBL_UNIFORM_SIZE];
            data[0..4].copy_from_slice(&self.ibl.prefiltered_env.mip_count.to_ne_bytes());
            data[4..8].copy_from_slice(&self.ibl.irradiance.mip_count.to_ne_bytes());

            gpu.write_buffer(&self.gpu_ibl_buffer, None, &*data, None, None)?;

            self.ibl_uniform_gpu_dirty = false;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Light {
    Directional {
        color: [f32; 3],
        intensity: f32,
        direction: [f32; 3],
    },
    Point {
        color: [f32; 3],
        intensity: f32,
        position: [f32; 3],
        range: f32,
    },
    Spot {
        color: [f32; 3],
        intensity: f32,
        position: [f32; 3],
        direction: [f32; 3],
        range: f32,
        inner_angle: f32,
        outer_angle: f32,
    },
}

impl Light {
    pub const BYTE_SIZE: usize = 64;

    pub fn enum_value(&self) -> f32 {
        // delibarately do not use 0
        // since removed lights will be zeroed out in memory
        // so 0 is reserved for "no light"
        match self {
            Light::Directional { .. } => 1.0,
            Light::Point { .. } => 2.0,
            Light::Spot { .. } => 3.0,
        }
    }
    pub fn storage_buffer_data(&self) -> [u8; Self::BYTE_SIZE] {
        let mut data = [0u8; Self::BYTE_SIZE];
        let mut offset = 0;

        #[derive(Debug)]
        enum Value<'a> {
            F32(&'a f32),
            Vec3(&'a [f32; 3]),
            SkipVec3,
            SkipN32(usize),
        }

        impl<'a> From<&'a f32> for Value<'a> {
            fn from(value: &'a f32) -> Self {
                Value::F32(value)
            }
        }

        impl<'a> From<&'a [f32; 3]> for Value<'a> {
            fn from(value: &'a [f32; 3]) -> Self {
                Value::Vec3(value)
            }
        }

        let mut write = |value: Value| match value {
            Value::F32(value) => {
                let bytes = value.to_ne_bytes();
                data[offset..offset + 4].copy_from_slice(&bytes);
                offset += 4;
            }
            Value::Vec3(values) => {
                let values_u8 =
                    unsafe { std::slice::from_raw_parts(values.as_ptr() as *const u8, 12) };
                data[offset..offset + 12].copy_from_slice(values_u8);
                offset += 12;
            }
            Value::SkipVec3 => {
                offset += 12;
            }
            Value::SkipN32(count) => {
                offset += 4 * count;
            }
        };

        // Layout is:
        // vec4<f32>(light_type, color.rgb)
        // vec4<f32>(intensity, position.xyz)
        // vec4<f32>(range, direction.xyz)
        // vec4<f32>(inner_angle, outer_angle, 0.0, 0.0)

        match self {
            Light::Directional {
                color,
                intensity,
                direction,
            } => {
                // row 1
                write((&self.enum_value()).into()); // light type
                write(color.into());
                // row 2
                write(intensity.into());
                write(Value::SkipVec3); // skip position
                                        // row 3
                write(Value::SkipN32(1)); // skip range
                write(direction.into());
                // row 4
                write(Value::SkipN32(4)); // skip all
            }
            Light::Point {
                color,
                intensity,
                position,
                range,
            } => {
                // row 1
                write((&self.enum_value()).into()); // light type
                write(color.into());

                // row 2
                write(intensity.into());
                write(position.into());
                // row 3
                write(range.into());
                // row 4 (and direction)
                write(Value::SkipN32(5)); // skip direction and all of row 4
            }
            Light::Spot {
                color,
                intensity,
                position,
                direction,
                range,
                inner_angle,
                outer_angle,
            } => {
                // row 1
                write((&self.enum_value()).into()); // light type
                write(color.into());
                // row 2
                write(intensity.into());
                write(position.into());
                // row 3
                write(range.into());
                write(direction.into());
                // row 4
                write(inner_angle.into());
                write(outer_angle.into());
                write(Value::SkipN32(2)); // skip end padding
            }
        }

        data
    }
}

new_key_type! {
    pub struct LightKey;
}

type Result<T> = std::result::Result<T, AwsmLightError>;

#[derive(Error, Debug)]
pub enum AwsmLightError {
    #[error("[light] {0:?}")]
    Core(#[from] AwsmCoreError),
}
