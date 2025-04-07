use awsm_renderer_core::buffer::{BufferDescriptor, BufferUsage};
use awsm_renderer_core::error::AwsmCoreError;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::{Mat4, Vec3};
use thiserror::Error;

pub struct CameraBuffer {
    buffer: web_sys::GpuBuffer,
    gpu: AwsmRendererWebGpu,
}

pub trait CameraExt {
    fn projection_matrix(&self) -> Mat4;

    fn view_matrix(&self) -> Mat4;

    fn position_world(&self) -> Vec3;
}

impl CameraBuffer {
    pub fn new(gpu: AwsmRendererWebGpu) -> Result<Self> {
        let buffer = gpu
            .create_buffer(
                &BufferDescriptor::new(
                    Some("Camera"),
                    16 * 4,
                    BufferUsage::new().with_uniform().with_copy_dst(),
                )
                .into(),
            )
            .map_err(AwsmCameraError::CreateBuffer)?;

        Ok(Self { gpu, buffer })
    }

    pub fn write(&self, camera: &impl CameraExt) -> Result<()> {
        let data = get_buffer_array(camera);

        self.gpu
            .write_buffer(&self.buffer, None, data.as_slice(), None, None)
            .map_err(AwsmCameraError::WriteBuffer)?;

        Ok(())
    }
}

// combine all the camera data into a single u8 array of bytes
fn get_buffer_array(camera: &impl CameraExt) -> [u8; 336] {
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

    let mut data = [0; 336];
    let mut offset = 0;

    let mut write_to_data = |values: &[f32]| {
        let len = values.len() * 4;

        let values_u8 = unsafe { std::slice::from_raw_parts(values.as_ptr() as *const u8, len) };

        data[offset..offset + len].copy_from_slice(values_u8);

        offset += len;
    };

    write_to_data(&view.to_cols_array());
    write_to_data(&proj.to_cols_array());
    write_to_data(&view_proj.to_cols_array());
    write_to_data(&inv_view_proj.to_cols_array());
    write_to_data(&inv_view.to_cols_array());
    write_to_data(&position.to_array());

    data
}

type Result<T> = std::result::Result<T, AwsmCameraError>;

#[derive(Error, Debug)]
pub enum AwsmCameraError {
    #[error("[camera] Error creating buffer")]
    CreateBuffer(AwsmCoreError),

    #[error("[camera] Error writing buffer")]
    WriteBuffer(AwsmCoreError),
}
