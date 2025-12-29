use awsm_renderer_core::buffers::{BufferDescriptor, BufferUsage};
use awsm_renderer_core::error::AwsmCoreError;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use glam::{Mat4, Vec2, Vec3, Vec4};
use thiserror::Error;

use crate::bind_groups::BindGroups;
use crate::render_textures::RenderTextures;
use crate::{AwsmRenderer, AwsmRendererLogging};

const APPLY_JITTER: bool = false;

impl AwsmRenderer {
    pub fn update_camera(&mut self, camera_matrices: CameraMatrices) -> Result<()> {
        let (current_width, current_height) = self.gpu.current_context_texture_size()?;

        self.camera.update(
            camera_matrices,
            &self.render_textures,
            current_width as f32,
            current_height as f32,
        )?;

        Ok(())
    }
}

pub struct CameraBuffer {
    pub(crate) raw_data: [u8; Self::BYTE_SIZE],
    pub gpu_buffer: web_sys::GpuBuffer,
    pub last_matrices: Option<CameraMatrices>,
    camera_moved: bool,
    gpu_dirty: bool,
}

#[derive(Clone, Debug)]
pub struct CameraMatrices {
    pub view: Mat4,
    pub projection: Mat4,
    pub position_world: Vec3,
}

impl CameraMatrices {
    pub fn view_projection(&self) -> Mat4 {
        self.projection * self.view
    }

    pub fn inv_view_projection(&self) -> Mat4 {
        self.view_projection().inverse()
    }
}

impl CameraBuffer {
    // Layout (tightly packed, no implicit padding):
    //  view                (mat4)  64 bytes
    //  projection          (mat4)  64 bytes
    //  view_projection     (mat4)  64 bytes
    //  inv_view_projection (mat4)  64 bytes
    //  inv_projection      (mat4)  64 bytes
    //  inv_view            (mat4)  64 bytes
    //  position (vec4, w=unused) 16 bytes
    //  frame_count_and_padding (vec4<u32>) 16 bytes
    //  frustum corner rays (4 * vec4) 64 bytes
    //  padding (2 * vec4) 32 bytes
    // Total = 512 bytes (all members 16-byte aligned, no implicit gaps)
    pub const BYTE_SIZE: usize = 512;

    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let gpu_buffer = gpu.create_buffer(
            &BufferDescriptor::new(
                Some("Camera"),
                Self::BYTE_SIZE,
                BufferUsage::new().with_uniform().with_copy_dst(),
            )
            .into(),
        )?;

        Ok(Self {
            raw_data: [0; Self::BYTE_SIZE],
            gpu_dirty: true,
            last_matrices: None,
            camera_moved: false,
            gpu_buffer,
        })
    }

    // this is fast/cheap to call, so we can call it multiple times a frame
    // it will only update the data in the buffer once per frame, at render time
    pub(crate) fn update(
        &mut self,
        camera_matrices_orig: CameraMatrices,
        render_textures: &RenderTextures,
        screen_width: f32,
        screen_height: f32,
    ) -> Result<()> {
        let mut camera_matrices = camera_matrices_orig.clone();

        self.camera_moved = match &self.last_matrices {
            Some(last_matrices) => {
                fn matrices_equal(a: Mat4, b: Mat4, epsilon: f32) -> bool {
                    for i in 0..16 {
                        if (a.to_cols_array()[i] - b.to_cols_array()[i]).abs() > epsilon {
                            return false;
                        }
                    }
                    true
                }
                // Check if matrices changed (with small epsilon for floating point comparison)
                !matrices_equal(last_matrices.view, camera_matrices.view, 1e-6)
                    || !matrices_equal(last_matrices.projection, camera_matrices.projection, 1e-6)
            }
            _ => true, // First frame, assume movement
        };

        if APPLY_JITTER {
            let jitter_strength = if self.camera_moved { 0.2 } else { 0.8 };
            // TAA jitter
            let jitter = get_halton_jitter(render_textures.frame_count());
            let jitter_ndc_x = (jitter.x / screen_width) * jitter_strength;
            let jitter_ndc_y = (jitter.y / screen_height) * jitter_strength;

            // Create jitter translation matrix
            let jitter_matrix = Mat4::from_translation(Vec3::new(jitter_ndc_x, jitter_ndc_y, 0.0));

            // Apply to your projection matrix
            camera_matrices.projection = jitter_matrix * camera_matrices.projection;
        }

        // Layout written below (mirrors `CameraUniform` in WGSL). The additional inverse
        // projection and frustum rays let compute passes reconstruct per-pixel view/world
        // positions directly from the depth buffer.
        //
        // IMPORTANT: frustum_rays are for SCREEN-SPACE RECONSTRUCTION, NOT frustum culling!
        // They are 4 normalized view-space ray directions at the near plane corners,
        // used for unprojecting screen pixels to world space (deferred rendering, grids, etc.).
        // For frustum culling, you need 6 frustum planes extracted from the view-proj matrix.

        let inv_projection = camera_matrices.projection.inverse();
        let inv_view_projection = camera_matrices.inv_view_projection();
        let inv_view = camera_matrices.view.inverse();
        let frustum_rays = compute_view_frustum_rays(inv_projection);

        // let s = format!("CameraBuffer Update, inv_projection: {inv_projection:?} inv_view_projection: {inv_view_projection:?} inv_view: {inv_view:?} frustum rays: {frustum_rays:?}");

        // debug_unique_string(1, &s, || tracing::info!("{s}"));

        let mut offset = 0;

        let view = camera_matrices.view.to_cols_array();
        write_f32_slice(&mut self.raw_data, &mut offset, &view);
        let projection = camera_matrices.projection.to_cols_array();
        write_f32_slice(&mut self.raw_data, &mut offset, &projection);
        let view_projection = camera_matrices.view_projection().to_cols_array();
        write_f32_slice(&mut self.raw_data, &mut offset, &view_projection);
        let inv_view_projection_cols = inv_view_projection.to_cols_array();
        write_f32_slice(&mut self.raw_data, &mut offset, &inv_view_projection_cols);
        let inv_projection_cols = inv_projection.to_cols_array();
        write_f32_slice(&mut self.raw_data, &mut offset, &inv_projection_cols);
        let inv_view_cols = inv_view.to_cols_array();
        write_f32_slice(&mut self.raw_data, &mut offset, &inv_view_cols);
        // Write position as vec4 (xyz + unused w component)
        let position = camera_matrices.position_world.extend(0.0).to_array();
        write_f32_slice(&mut self.raw_data, &mut offset, &position);
        // Write frame_count_and_padding as vec4<u32> (x = frame_count, yzw = padding)
        write_u32(
            &mut self.raw_data,
            &mut offset,
            render_textures.frame_count(),
        );
        write_u32(&mut self.raw_data, &mut offset, 0);
        write_u32(&mut self.raw_data, &mut offset, 0);
        write_u32(&mut self.raw_data, &mut offset, 0);

        for ray in frustum_rays.iter() {
            let ray_values = ray.to_array();
            write_f32_slice(&mut self.raw_data, &mut offset, &ray_values);
        }

        // Struct alignment padding (32 bytes at end) - WGSL compute pipeline requirement
        write_f32_slice(&mut self.raw_data, &mut offset, &[0.0, 0.0, 0.0, 0.0]);
        write_f32_slice(&mut self.raw_data, &mut offset, &[0.0, 0.0, 0.0, 0.0]);

        debug_assert_eq!(offset, Self::BYTE_SIZE, "Buffer layout mismatch!");

        self.gpu_dirty = true;

        // Store for next frame (unjittered versions)
        self.last_matrices = Some(camera_matrices_orig);

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
        _bind_groups: &BindGroups,
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

/// Compute 4 normalized view-space ray directions for the near plane corners.
///
/// These rays are used for screen-space reconstruction (unprojecting screen pixels to world space).
/// Shaders bilinearly interpolate these corner rays to get the ray direction for any pixel,
/// providing better numerical precision than doing full unprojection per-pixel.
///
/// **NOT for frustum culling** - culling needs 6 frustum planes extracted from view-proj matrix.
///
/// Order: [0]=bottom-left, [1]=bottom-right, [2]=top-left, [3]=top-right
fn compute_view_frustum_rays(inv_projection: Mat4) -> [Vec4; 4] {
    // Reproject the clip-space corners of the near plane back into view space. These serve as
    // canonical ray directions that the compute shader can bilinearly interpolate per pixel.
    // Use z=0 (near plane in WebGPU NDC), not z=1 (far plane) to avoid infinities
    let ndc_corners = [
        Vec4::new(-1.0, -1.0, 0.0, 1.0),
        Vec4::new(1.0, -1.0, 0.0, 1.0),
        Vec4::new(-1.0, 1.0, 0.0, 1.0),
        Vec4::new(1.0, 1.0, 0.0, 1.0),
    ];

    let mut rays = [Vec4::ZERO; 4];
    for (i, corner) in ndc_corners.iter().enumerate() {
        let view_space = inv_projection * *corner;
        let view_space = view_space / view_space.w;
        // Normalize to get ray direction (not position)
        let ray_dir = Vec3::new(view_space.x, view_space.y, view_space.z).normalize();
        rays[i] = Vec4::new(ray_dir.x, ray_dir.y, ray_dir.z, 0.0);
    }

    rays
}

fn write_f32_slice(buffer: &mut [u8], offset: &mut usize, values: &[f32]) {
    // All matrices/vectors in the camera buffer are tightly packed f32 arrays. Writing them this
    // way keeps the CPU-side layout authoritative and avoids duplicating offset math.
    let byte_len = std::mem::size_of_val(values);

    // crate::debug::debug_unique_string(*offset as u32, &format!("{:?}", values), || {
    //     tracing::info!("[{}]: {:?}", offset, values);
    // });

    let bytes = unsafe { std::slice::from_raw_parts(values.as_ptr() as *const u8, byte_len) };
    buffer[*offset..*offset + byte_len].copy_from_slice(bytes);
    *offset += byte_len;
}

fn write_u32(buffer: &mut [u8], offset: &mut usize, value: u32) {
    // WGSL requires 16-byte alignment. We store the frame counter alongside the camera position,
    // treating it as a padded vec4 on the shader side.
    buffer[*offset..*offset + 4].copy_from_slice(&value.to_ne_bytes());
    *offset += 4;
}

type Result<T> = std::result::Result<T, AwsmCameraError>;

#[derive(Error, Debug)]
pub enum AwsmCameraError {
    #[error("[camera] {0:?}")]
    Core(#[from] AwsmCoreError),
}
