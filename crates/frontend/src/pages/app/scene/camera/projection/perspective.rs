use awsm_renderer::bounds::Aabb;
use glam::{Mat4, Vec3};

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

    /// Recomputes near/far after camera view changes (orbit/pan/rotate).
    pub fn on_view_changed(&mut self, view: &CameraView, aabb: &Aabb, margin: f32) {
        self.update_near_far(view, aabb, margin);
    }

    /// Keeps the scene fully inside the frustum while minimising
    /// depth‑buffer precision loss.
    fn update_near_far(&mut self, view: &CameraView, aabb: &Aabb, margin: f32) {
        let (near, far) = tight_clip_planes_from_aabb(view, aabb, margin);
        self.near = near;
        self.far = far;
    }

    /// Standard right‑handed perspective projection
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    pub fn setup_from_gltf(&mut self, _doc: &gltf::Document) {}
}

fn tight_clip_planes_from_aabb(view: &CameraView, aabb: &Aabb, margin: f32) -> (f32, f32) {
    let view_matrix = view.view_matrix();

    let corners = [
        Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z),
        Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z),
        Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z),
        Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z),
        Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z),
        Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z),
        Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z),
        Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z),
    ];

    // Camera looks down -Z in view space, so forward distance is -z.
    let mut min_d = f32::INFINITY;
    let mut max_d = f32::NEG_INFINITY;
    for corner in &corners {
        let v = view_matrix.transform_point3(*corner);
        let d = -v.z;
        min_d = min_d.min(d);
        max_d = max_d.max(d);
    }

    // Apply symmetric slack around the visible depth range.
    let center = (min_d + max_d) * 0.5;
    let half = ((max_d - min_d) * 0.5 * margin).max(0.001);
    let mut near = center - half;
    let mut far = center + half;

    const MIN_NEAR: f32 = 0.001;
    const MIN_RANGE: f32 = 0.1;
    const MAX_DEPTH_RATIO: f32 = 1_000_000_000.0;

    near = near.max(MIN_NEAR);
    far = far.max(near + MIN_RANGE);

    // Conservative fallback to avoid clipping animated/poorly-bounded scenes.
    // This intentionally favors visibility over depth precision.
    let view_distance = (view.position() - view.look_at()).length();
    let scene_radius = (aabb.size().length() * 0.5 * margin.max(1.0)).max(1.0);
    let conservative_far = (view_distance + scene_radius * 256.0).max(10_000.0);
    near = MIN_NEAR;
    far = far.max(conservative_far);

    // Keep far/near ratio bounded to preserve depth precision.
    let ratio_near = far / MAX_DEPTH_RATIO;
    if near < ratio_near {
        near = ratio_near.max(MIN_NEAR);
    }

    (near, far)
}
