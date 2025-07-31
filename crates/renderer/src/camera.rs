use awsm_renderer_core::buffers::{BufferDescriptor, BufferUsage};
use awsm_renderer_core::error::AwsmCoreError;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::{Mat4, Vec2, Vec3};
use thiserror::Error;

use crate::bind_groups::{AwsmBindGroupError, BindGroups};
use crate::render_textures::RenderTextures;
use crate::{AwsmRenderer, AwsmRendererLogging};

impl AwsmRenderer {
    pub fn update_camera(&mut self, camera: &impl CameraExt) -> Result<()> {
        let (current_width, current_height) = self
            .gpu
            .current_context_texture_size()?;
        self.camera.update(
            camera,
            &self.render_textures,
            true,
            current_width as f32,
            current_height as f32,
        )?;

        Ok(())
    }
}

pub struct CameraBuffer {
    pub(crate) raw_data: [u8; Self::BYTE_SIZE],
    pub(crate) gpu_buffer: web_sys::GpuBuffer,
    last_view_matrix: Option<Mat4>,
    last_proj_matrix: Option<Mat4>,
    camera_moved: bool,
    gpu_dirty: bool,
}

pub trait CameraExt {
    fn projection_matrix(&self) -> Mat4;

    fn view_matrix(&self) -> Mat4;

    fn position_world(&self) -> Vec3;
}

impl CameraBuffer {
    pub const BYTE_SIZE: usize = 336; // see `update()` for details

    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let gpu_buffer = gpu.create_buffer(&BufferDescriptor::new(
            Some("Camera"),
            Self::BYTE_SIZE,
            BufferUsage::new().with_uniform().with_copy_dst()
        ).into())?;

        Ok(Self {
            raw_data: [0; Self::BYTE_SIZE],
            gpu_dirty: true,
            last_view_matrix: None,
            last_proj_matrix: None,
            camera_moved: false,
            gpu_buffer,
        })
    }

    // this is fast/cheap to call, so we can call it multiple times a frame
    // it will only update the data in the buffer once per frame, at render time
    pub(crate) fn update(
        &mut self,
        camera: &impl CameraExt,
        render_textures: &RenderTextures,
        apply_jitter: bool,
        screen_width: f32,
        screen_height: f32,
    ) -> Result<()> {
        let view = camera.view_matrix(); // 16 floats
        let mut proj = camera.projection_matrix(); // 16 floats

        self.camera_moved = match (&self.last_view_matrix, &self.last_proj_matrix) {
            (Some(last_view), Some(last_proj)) => {
                fn matrices_equal(a: Mat4, b: Mat4, epsilon: f32) -> bool {
                    for i in 0..16 {
                        if (a.to_cols_array()[i] - b.to_cols_array()[i]).abs() > epsilon {
                            return false;
                        }
                    }
                    true
                }
                // Check if matrices changed (with small epsilon for floating point comparison)
                !matrices_equal(*last_view, view, 1e-6) || !matrices_equal(*last_proj, proj, 1e-6)
            }
            _ => true, // First frame, assume movement
        };

        if apply_jitter {
            let jitter_strength = if self.camera_moved {
                0.2
            } else {
                0.8
            };
            // TAA jitter
            let jitter = get_halton_jitter(render_textures.frame_count());
            let jitter_ndc_x = (jitter.x / screen_width) * jitter_strength;
            let jitter_ndc_y = (jitter.y / screen_height) * jitter_strength;

            // Create jitter translation matrix
            let jitter_matrix = Mat4::from_translation(Vec3::new(jitter_ndc_x, jitter_ndc_y, 0.0));

            // Apply to your projection matrix
            proj = jitter_matrix * proj;
        }

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
        self.raw_data[offset..offset + 4]
            .copy_from_slice(&render_textures.frame_count().to_ne_bytes());

        self.gpu_dirty = true;

        // Store for next frame (unjittered versions)
        self.last_view_matrix = Some(camera.view_matrix());
        self.last_proj_matrix = Some(camera.projection_matrix());

        Ok(())
    }

    pub fn moved(&self) -> bool {
        self.camera_moved
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

            gpu.write_buffer(&self.gpu_buffer, None, self.raw_data.as_slice(), None, None)?;

            self.gpu_dirty = false;
        }

        Ok(())
    }
}
fn get_halton_jitter(frame_count: u32) -> Vec2 {
    let x = halton(frame_count, 2) - 0.5;
    let y = halton(frame_count, 3) - 0.5;
    Vec2::new(x, y)
}

fn halton(mut index: u32, base: u32) -> f32 {
    let mut result = 0.0;
    let mut f = 1.0;

    while index > 0 {
        f /= base as f32;
        result += f * (index % base) as f32;
        index /= base;
    }

    result
}

type Result<T> = std::result::Result<T, AwsmCameraError>;

#[derive(Error, Debug)]
pub enum AwsmCameraError {
    #[error("[camera] {0:?}")]
    Core(#[from] AwsmCoreError),
}
