use awsm_renderer::mesh::PositionExtents;
use glam::{Mat4, Vec3, Vec4};

/// Orthographic projection camera for WebGPU (depth range [0,1])
#[derive(Debug, Clone)]
pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl OrthographicCamera {
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        )
    }

    /// Zooms the ortho view in (factor<1) or out (factor>1), keeping the same center.
    pub fn zoom(&mut self, factor: f32) {
        let cx = (self.left + self.right) * 0.5;
        let cy = (self.bottom + self.top) * 0.5;
        let half_w = (self.right - self.left) * 0.5 * factor;
        let half_h = (self.top - self.bottom) * 0.5 * factor;

        self.left = cx - half_w;
        self.right = cx + half_w;
        self.bottom = cy - half_h;
        self.top = cy + half_h;
    }
}
