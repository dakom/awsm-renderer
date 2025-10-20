use awsm_renderer::bounds::Aabb;
use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone)]
pub struct OrbitCamera {
    /// Point the camera orbits around
    pub look_at: Vec3,
    /// Distance from look_at
    pub radius: f32,
    pub sensitivity: f32,

    // internal state for yaw/pitch orbiting
    yaw: f32,
    pitch: f32,
    dragging: bool,
}

impl OrbitCamera {
    pub fn new_aabb(aabb: &Aabb, margin: f32) -> Self {
        let center = aabb.center();
        let size = aabb.size();

        let bounding_radius = size.length() * 0.5;
        let radius = bounding_radius * margin;

        OrbitCamera::new(center, radius)
    }

    pub fn new(look_at: Vec3, radius: f32) -> Self {
        Self {
            look_at,
            radius,
            // Start head-on: looking from +Z axis, slightly above
            // yaw: 0 = looking from +Z, π/2 = from +X, π = from -Z, 3π/2 = from -X
            yaw: 0.0, // Head-on view from +Z
            // pitch: positive = camera above looking down
            pitch: 0.3, // ~17° above horizon, looking down slightly
            dragging: false,
            sensitivity: 0.005,
        }
    }

    /// Returns a right-handed look-at view matrix
    pub fn get_view_matrix(&self) -> Mat4 {
        let cam_pos = self.get_position();
        Mat4::look_at_rh(cam_pos, self.look_at, Vec3::Y)
    }

    /// Returns the current camera world position (Vec3)
    /// Uses spherical coordinates: yaw (horizontal angle), pitch (vertical angle), radius (distance)
    pub fn get_position(&self) -> Vec3 {
        // Spherical to Cartesian conversion
        // pitch: angle from XZ plane (0 = horizon, positive = above, negative = below)
        // yaw: angle around Y axis (0 = +Z, π/2 = +X, π = -Z, 3π/2 = -X)
        let x = self.radius * self.pitch.cos() * self.yaw.sin();
        let y = self.radius * self.pitch.sin();
        let z = self.radius * self.pitch.cos() * self.yaw.cos();

        self.look_at + Vec3::new(x, y, z)
    }

    pub fn on_pointer_down(&mut self) {
        self.dragging = true;
    }

    pub fn on_pointer_move(&mut self, delta_x: f32, delta_y: f32) {
        if !self.dragging {
            return;
        }

        self.yaw -= delta_x * self.sensitivity;
        self.pitch -= delta_y * self.sensitivity;

        // Clamp pitch to ±90° like most 3D viewers
        // This prevents flipping and matches Khronos glTF viewer behavior
        let limit = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = self.pitch.clamp(-limit, limit);
    }

    pub fn on_pointer_up(&mut self) {
        self.dragging = false;
    }

    pub fn on_wheel(&mut self, delta_y: f32) {
        let zoom_factor = 1.0 + delta_y * 0.001;
        self.radius = (self.radius * zoom_factor).max(0.1);
    }

    pub fn setup_from_gltf(&mut self, doc: &gltf::Document) {
        // TODO: Implement proper camera orientation detection based on glTF scene data
        // For now, use consistent defaults and let users rotate manually
    }
}
