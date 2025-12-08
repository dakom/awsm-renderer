use awsm_renderer::bounds::Aabb;
use glam::Mat4;

use crate::pages::app::scene::camera::CameraView;

/// Perspective projection camera for WebGPU (depth range [0, 1])
#[derive(Debug, Clone)]
pub struct PerspectiveCamera {
    /// Vertical field‑of‑view in radians
    pub fov_y: f32,
    /// Viewport aspect ratio (width / height)
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl PerspectiveCamera {
    /// Build a perspective camera that encloses `aabb` with the same
    /// margin logic used for the orthographic version.
    ///
    /// `aspect` should come from the current swap‑chain size.
    pub fn new_aabb(view: &CameraView, aabb: &Aabb, margin: f32, aspect: f32) -> Self {
        // 45° vertical FOV is a comfortable default
        let fov_y = 45.0_f32.to_radians();

        let mut this = Self {
            fov_y,
            aspect,
            near: 0.01, // placeholder – fixed just below
            far: 100.0, // placeholder – fixed just below
        };

        this.update_near_far(view, aabb, margin);
        this
    }

    /// Call whenever the window is resized.
    pub fn on_resize(&mut self, new_aspect: f32) {
        self.aspect = new_aspect;
    }

    /// Call on every mouse‑wheel event *after* `OrbitCamera` has
    /// updated its radius, so the clip planes stay snug.
    pub fn on_wheel(&mut self, view: &CameraView, aabb: &Aabb, margin: f32) {
        self.update_near_far(view, aabb, margin);
    }

    /// Keeps the scene fully inside the frustum while minimising
    /// depth‑buffer precision loss.
    fn update_near_far(&mut self, view: &CameraView, aabb: &Aabb, margin: f32) {
        let bounding_radius = aabb.size().length() * 0.5;
        let distance = view.position().distance(view.look_at());

        // Give ourselves a little slack in front and behind
        self.near = (distance - bounding_radius * margin * 2.0).max(0.01);
        self.far = distance + bounding_radius * margin * 2.0;

        // eh, whatever
        self.near = self.near.min(0.001);
        self.far = self.far.max(1000000.0);
    }

    /// Standard right‑handed perspective projection
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    pub fn setup_from_gltf(&mut self, _doc: &gltf::Document) {}
}
