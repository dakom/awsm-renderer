use awsm_renderer_core::error::AwsmCoreError;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::{Mat4, Vec3};
use thiserror::Error;

use crate::bind_groups::{
    buffer::BufferBindGroupIndex, buffer::UniversalBindGroupBinding, AwsmBindGroupError, BindGroups,
};
use crate::{AwsmRenderer, AwsmRendererLogging};

impl AwsmRenderer {
    pub fn update_camera(&mut self, camera: &impl CameraExt) -> Result<()> {
        self.camera.update(camera)
    }
}

pub struct CameraBuffer {
    pub(crate) raw_data: [u8; Self::BYTE_SIZE],
    gpu_dirty: bool,
}

pub trait CameraExt {
    fn projection_matrix(&self) -> Mat4;

    fn view_matrix(&self) -> Mat4;

    fn position_world(&self) -> Vec3;
}

impl CameraBuffer {
    pub const BYTE_SIZE: usize = 336; // see `update()` for details

    pub fn new() -> Result<Self> {
        Ok(Self {
            raw_data: [0; Self::BYTE_SIZE],
            gpu_dirty: true,
        })
    }

    // this is fast/cheap to call, so we can call it multiple times a frame
    // it will only update the data in the buffer once per frame, at render time
    pub(crate) fn update(&mut self, camera: &impl CameraExt) -> Result<()> {
        let view = camera.view_matrix(); // 16 floats
        let proj = camera.projection_matrix(); // 16 floats

        let view_proj = proj * view; // 16 floats
        let inv_view_proj = view_proj.inverse(); // 16 floats
        let inv_view = view.inverse(); // 16 floats

        let position = camera.position_world(); // 3 floats

        // altogether that's 83 floats: (16*5) + 3
        // or 332 bytes: 83 * 4
        // however, we need to pad it to a multiple of 16 (https://www.w3.org/TR/WGSL/#address-space-layout-constraints)
        // so we need to add 4 bytes of padding (this will effectively make the `position` a vec4 instead of vec3 in wgsl side)
        // 332 + 4 = 336

        let mut offset = 0;

        let mut write_to_data = |values: &[f32]| {
            let len = values.len() * 4;

            let values_u8 =
                unsafe { std::slice::from_raw_parts(values.as_ptr() as *const u8, len) };

            self.raw_data[offset..offset + len].copy_from_slice(values_u8);

            offset += len;
        };

        write_to_data(&view.to_cols_array());
        write_to_data(&proj.to_cols_array());
        write_to_data(&view_proj.to_cols_array());
        write_to_data(&inv_view_proj.to_cols_array());
        write_to_data(&inv_view.to_cols_array());
        write_to_data(&position.to_array());

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
                Some(tracing::span!(tracing::Level::INFO, "Camera GPU write").entered())
            } else {
                None
            };

            bind_groups
                .buffers
                .gpu_write(
                    gpu,
                    BufferBindGroupIndex::Universal(UniversalBindGroupBinding::Camera),
                    None,
                    self.raw_data.as_slice(),
                    None,
                    None,
                )
                .map_err(AwsmCameraError::WriteBuffer)?;
            self.gpu_dirty = false;
        }

        Ok(())
    }
}

type Result<T> = std::result::Result<T, AwsmCameraError>;

#[derive(Error, Debug)]
pub enum AwsmCameraError {
    #[error("[camera] Error creating buffer: {0:?}")]
    CreateBuffer(AwsmCoreError),

    #[error("[camera] Error writing buffer: {0:?}")]
    WriteBuffer(AwsmBindGroupError),
}
