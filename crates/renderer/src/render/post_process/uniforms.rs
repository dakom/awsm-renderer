use awsm_renderer_core::renderer::AwsmRendererWebGpu;

use crate::{bind_groups::{uniform_storage::{PostProcessBindGroupBinding, UniformStorageBindGroupIndex}, BindGroups}, AwsmRendererLogging};

use super::error::{AwsmPostProcessError, Result};
pub struct PostProcessUniforms {
    pub(crate) raw_data: [u8; Self::BYTE_SIZE],
    gpu_dirty: bool,
    ping_pong: bool,
}

impl PostProcessUniforms {
    pub const BYTE_SIZE: usize = 32; // see `update()` for details

    pub fn new() -> Self {
        Self {
            raw_data: [0; Self::BYTE_SIZE],
            gpu_dirty: true,
            ping_pong: false,
        }
    }

    pub fn toggle_ping_pong(&mut self) -> Result<bool> {
        self.ping_pong = !self.ping_pong;
        self.update()?;

        Ok(self.ping_pong)
    }

    // this is fast/cheap to call, so we can call it multiple times a frame
    // it will only update the data in the buffer once per frame, at render time
    fn update(&mut self) -> Result<()> {
        let mut offset = 0;

        let mut write_bool = |value: bool| {
            if value {
                self.raw_data[offset..offset + 4].copy_from_slice(&1u32.to_ne_bytes());
            } else {
                self.raw_data[offset..offset + 4].copy_from_slice(&0u32.to_ne_bytes());
            }

            offset += 4;
        };

        write_bool(self.ping_pong);

        self.gpu_dirty = true;

        Ok(())
    }

    // writes to the GPU
    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &BindGroups,
    ) -> Result<()> {
        if self.gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "Post processing GPU write").entered())
            } else {
                None
            };

            bind_groups
                .uniform_storages
                .gpu_write(
                    gpu,
                    UniformStorageBindGroupIndex::PostProcess(PostProcessBindGroupBinding::Settings),
                    None,
                    self.raw_data.as_slice(),
                    None,
                    None,
                )
                .map_err(AwsmPostProcessError::WriteBuffer)?;
            self.gpu_dirty = false;
        }

        Ok(())
    }
}