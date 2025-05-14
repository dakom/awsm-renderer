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
            yaw: 0.0,
            pitch: 0.0,
            dragging: false,
            sensitivity: 0.005,
        }
    }

    /// Returns a right-handed look-at view matrix
    pub fn get_view_matrix(&self) -> Mat4 {
        let q_yaw = Quat::from_rotation_y(self.yaw);
        let right = q_yaw * Vec3::X;
        let q_pitch = Quat::from_axis_angle(right, self.pitch);
        let rotation = q_pitch * q_yaw;

        let cam_pos = self.look_at + rotation * Vec3::new(0.0, 0.0, self.radius);
        let up_dir = rotation * Vec3::Y;
        Mat4::look_at_rh(cam_pos, self.look_at, up_dir)
    }

    /// Returns the current camera world position (Vec3)
    pub fn get_position(&self) -> Vec3 {
        let rotation = Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch);
        let offset = rotation * Vec3::new(0.0, 0.0, self.radius);
        self.look_at + offset
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

        // clamp pitch so you can’t flip over top
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
}
