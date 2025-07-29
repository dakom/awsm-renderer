use awsm_renderer_core::renderer::AwsmRendererWebGpu;

use crate::{
    bind_groups::{
        uniform_storage::{PostProcessBindGroupBinding, UniformStorageBindGroupIndex},
        BindGroups,
    },
    AwsmRendererLogging,
};

use super::error::{AwsmPostProcessError, Result};
pub struct PostProcessUniforms {
    pub(crate) raw_data: [u8; Self::BYTE_SIZE],
    gpu_dirty: bool,
}

impl Default for PostProcessUniforms {
    fn default() -> Self {
        Self::new()
    }
}

impl PostProcessUniforms {
    pub const BYTE_SIZE: usize = 64; // see `update()` for details

    pub fn new() -> Self {
        Self {
            raw_data: [0; Self::BYTE_SIZE],
            gpu_dirty: true,
        }
    }

    // this is fast/cheap to call, so we can call it multiple times a frame
    // it will only update the data in the buffer once per frame, at render time
    pub fn update(&mut self, frame_count: u32, camera_moved: bool) -> Result<()> {
        let mut offset = 0;

        let mut write_u32 = |value: u32| {
            self.raw_data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
            offset += 4;
        };

        write_u32(frame_count);
        write_u32(if camera_moved { 1 } else { 0 });

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
                    UniformStorageBindGroupIndex::PostProcess(PostProcessBindGroupBinding::Data),
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
