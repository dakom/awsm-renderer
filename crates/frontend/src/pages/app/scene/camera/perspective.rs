use glam::{Mat4, Vec3};

#[derive(Debug, Clone)]
pub struct PerspectiveCamera {
    pub fovy_radians: f32, // vertical field of view (in radians)
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl PerspectiveCamera {
    pub fn projection_matrix(&self) -> Mat4 {
        // For WebGPU, use perspective_rh or perspective_lh (NOT perspective_rh_gl).
        Mat4::perspective_rh(self.fovy_radians, self.aspect_ratio, self.near, self.far)
    }

    pub fn set_extents(&mut self, min: Vec3, max: Vec3) {
        tracing::warn!("Perspective camera extents are not implemented yet");
    }

    pub fn set_canvas(&mut self, canvas: &web_sys::HtmlCanvasElement) {
        tracing::warn!("Perspective camera canvas is not implemented yet");
    }
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self {
            fovy_radians: std::f32::consts::FRAC_PI_4, // 45 degrees
            aspect_ratio: 800.0 / 600.0,
            near: 0.1,
            far: 100.0,
        }
    }
}
