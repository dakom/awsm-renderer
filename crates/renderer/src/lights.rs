use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::{
        uniform_storage::{UniformStorageBindGroupIndex, UniversalBindGroupBinding},
        AwsmBindGroupError, BindGroups,
    },
    buffer::dynamic_uniform::DynamicUniformBuffer,
    AwsmRendererLogging,
};

pub struct Lights {
    lights: SlotMap<LightKey, Light>,
    // we use it as a storage buffer, because we need dynamic lengths, but it's a fixed size like a uniform
    storage_buffer: DynamicUniformBuffer<LightKey>,
    gpu_dirty: bool,
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

impl Default for Lights {
    fn default() -> Self {
        Self::new()
    }
}

impl Lights {
    pub const INITIAL_ELEMENTS: usize = 8; // 8 lights is a decent baseline
    pub const BYTE_ALIGNMENT: usize = 64; // we aren't using it as a uniform buffer, so storage rules apply
    pub const BYTE_SIZE: usize = 64;
    pub fn new() -> Self {
        Lights {
            lights: SlotMap::with_key(),
            storage_buffer: DynamicUniformBuffer::new(
                Self::INITIAL_ELEMENTS,
                Self::BYTE_SIZE,
                Self::BYTE_ALIGNMENT,
                Some("Lights".to_string()),
            ),
            gpu_dirty: true,
        }
    }

    pub fn insert(&mut self, light: Light) -> Result<LightKey> {
        let key = self.lights.insert(light.clone());

        self.storage_buffer
            .update(key, &light.storage_buffer_data());

        self.gpu_dirty = true;
        Ok(key)
    }

    pub fn remove(&mut self, key: LightKey) {
        self.storage_buffer.remove(key);
        self.lights.remove(key);
        self.gpu_dirty = true;
    }

    pub fn update(&mut self, key: LightKey, f: impl FnOnce(&mut Light)) {
        if let Some(light) = self.lights.get_mut(key) {
            f(light);
            self.storage_buffer
                .update(key, &light.storage_buffer_data());
            self.gpu_dirty = true;
        }
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(
                    tracing::span!(tracing::Level::INFO, "Lights Storage Buffer GPU write")
                        .entered(),
                )
            } else {
                None
            };

            let bind_group_index =
                UniformStorageBindGroupIndex::Universal(UniversalBindGroupBinding::Lights);
            if let Some(new_size) = self.storage_buffer.take_gpu_needs_resize() {
                bind_groups
                    .uniform_storages
                    .gpu_resize(gpu, bind_group_index, new_size)
                    .map_err(AwsmLightError::BindGroupResize)?;
            }

            // for (index, chunk) in self.storage_buffer.raw_slice().chunks_exact(64).enumerate() {
            //     let values = unsafe {
            //         std::slice::from_raw_parts(chunk.as_ptr() as *const f32, 16)
            //     };
            //     tracing::info!("{}: {:?}", index, values);
            // }

            // tracing::info!("n_lights should be {}", self.storage_buffer.raw_slice().len() / (4 * 16));

            bind_groups
                .uniform_storages
                .gpu_write(
                    gpu,
                    bind_group_index,
                    None,
                    self.storage_buffer.raw_slice(),
                    None,
                    None,
                )
                .map_err(AwsmLightError::BindGroupWrite)?;

            self.gpu_dirty = false;
        }
        Ok(())
    }
}

new_key_type! {
    pub struct LightKey;
}

type Result<T> = std::result::Result<T, AwsmLightError>;

#[derive(Error, Debug)]
pub enum AwsmLightError {
    #[error("[light] unable to resize bind group: {0:?}")]
    BindGroupResize(AwsmBindGroupError),

    #[error("[light] unable to write bind group: {0:?}")]
    BindGroupWrite(AwsmBindGroupError),
}
