use awsm_renderer::bounds::Aabb;
use glam::{Mat4, Vec3, Vec4};

use crate::pages::app::scene::camera::CameraView;

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
    pub fn new_aabb(view: &CameraView, aabb: &Aabb, margin: f32, aspect: f32) -> Self {
        let size = aabb.size();

        let width = size.x;
        let height = size.y;
        //let aspect = width / height;
        let mut half_w = width * 0.5;
        let mut half_h = height * 0.5;

        if half_w / half_h > aspect {
            half_h = half_w / aspect;
        } else {
            half_w = half_h * aspect;
        }

        half_w *= margin;
        half_h *= margin;

        let mut _self = Self {
            left: -half_w,
            right: half_w,
            bottom: -half_h,
            top: half_h,
            near: 0.01, // initial placeholder
            far: 100.0, // initial placeholder
        };

        _self.on_resize(view, aabb, margin, aspect);

        _self
    }

    pub fn on_wheel(&mut self, view: &CameraView, aabb: &Aabb, margin: f32, delta: f32) {
        self.zoom(1.0 + delta as f32 * 0.001);
        self.update_near_far(view, aabb, margin);
    }

    // internal helper, whenever zoom changes to adjust clipping dynamically.
    fn update_near_far(&mut self, view: &CameraView, aabb: &Aabb, margin: f32) {
        let bounding_radius = aabb.size().length() * 0.5;

        let distance = view.position().distance(view.look_at());

        self.near = (distance - bounding_radius * margin * 2.0).max(0.01);
        self.far = distance + bounding_radius * margin * 2.0;
    }

    // Call this method whenever the window is resized.
    pub fn on_resize(&mut self, view: &CameraView, aabb: &Aabb, margin: f32, aspect: f32) {
        // current centre of the frustum
        let cx = (self.left + self.right) * 0.5;
        let cy = (self.bottom + self.top) * 0.5;

        // keep vertical span, change horizontal to match aspect
        let half_h = (self.top - self.bottom) * 0.5;
        let half_w = half_h * aspect;

        self.left = cx - half_w;
        self.right = cx + half_w;

        // near/far might change too if the user resized immediately
        // after zooming the orbit camera
        self.update_near_far(view, aabb, margin);
    }

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
