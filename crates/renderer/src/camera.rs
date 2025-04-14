use awsm_renderer_core::bind_groups::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindGroupLayoutResource, BindGroupResource, BufferBindingLayout, BufferBindingType,
};
use awsm_renderer_core::buffer::{BufferBinding, BufferDescriptor, BufferUsage};
use awsm_renderer_core::error::AwsmCoreError;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::{Mat4, Vec3};
use thiserror::Error;

use crate::shaders::BindGroupBinding;
use crate::AwsmRenderer;

impl AwsmRenderer {
    pub fn update_camera(&mut self, camera: &impl CameraExt) -> Result<()> {
        self.camera.update(camera)
    }
}

pub struct CameraBuffer {
    pub(crate) gpu_buffer: web_sys::GpuBuffer,
    pub(crate) raw_data: [u8; BUFFER_SIZE],
    pub bind_group: web_sys::GpuBindGroup,
    pub bind_group_layout: web_sys::GpuBindGroupLayout,
}

pub trait CameraExt {
    fn projection_matrix(&self) -> Mat4;

    fn view_matrix(&self) -> Mat4;

    fn position_world(&self) -> Vec3;
}

const BUFFER_SIZE: usize = 336; // see `update()` for details

impl CameraBuffer {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let gpu_buffer = gpu
            .create_buffer(
                &BufferDescriptor::new(
                    Some("Camera"),
                    BUFFER_SIZE,
                    BufferUsage::new().with_uniform().with_copy_dst(),
                )
                .into(),
            )
            .map_err(AwsmCameraError::CreateBuffer)?;

        let bind_group_layout = gpu
            .create_bind_group_layout(
                &BindGroupLayoutDescriptor::new(Some("Camera"))
                    .with_entries(vec![BindGroupLayoutEntry::new(
                        BindGroupBinding::Camera as u32,
                        BindGroupLayoutResource::Buffer(
                            BufferBindingLayout::new()
                                .with_binding_type(BufferBindingType::Uniform),
                        ),
                    )
                    .with_visibility_vertex()
                    .with_visibility_fragment()])
                    .into(),
            )
            .map_err(AwsmCameraError::CreateBindGroupLayout)?;

        let bind_group = gpu.create_bind_group(
            &BindGroupDescriptor::new(
                &bind_group_layout,
                Some("Camera"),
                vec![BindGroupEntry::new(
                    BindGroupBinding::Camera as u32,
                    BindGroupResource::Buffer(BufferBinding::new(&gpu_buffer)),
                )],
            )
            .into(),
        );

        Ok(Self {
            gpu_buffer,
            raw_data: [0; BUFFER_SIZE],
            bind_group,
            bind_group_layout,
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

        Ok(())
    }

    // writes to the GPU
    pub fn write_gpu(&self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        gpu.write_buffer(&self.gpu_buffer, None, self.raw_data.as_slice(), None, None)
            .map_err(AwsmCameraError::WriteBuffer)?;

        // TODO - transforms, etc.

        Ok(())
    }
}

type Result<T> = std::result::Result<T, AwsmCameraError>;

#[derive(Error, Debug)]
pub enum AwsmCameraError {
    #[error("[camera] Error creating buffer")]
    CreateBuffer(AwsmCoreError),

    #[error("[camera] Error writing buffer")]
    WriteBuffer(AwsmCoreError),

    #[error("[camera] Error creating bind group layout")]
    CreateBindGroupLayout(AwsmCoreError),
}
