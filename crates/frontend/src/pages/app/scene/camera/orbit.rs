use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone)]
pub struct OrbitCamera {
    /// Point the camera orbits around
    pub look_at: Vec3,
    /// Distance from look_at
    pub radius: f32,

    // internal state for yaw/pitch orbiting
    yaw: f32,
    pitch: f32,
    dragging: bool,
    last_x: f32,
    last_y: f32,
}

impl OrbitCamera {
    pub fn new(look_at: Vec3, radius: f32) -> Self {
        Self {
            look_at,
            radius,
            yaw: 0.0,
            pitch: 0.0,
            dragging: false,
            last_x: 0.0,
            last_y: 0.0,
        }
    }

    /// Returns a right-handed look-at view matrix
    pub fn get_view_matrix(&self) -> Mat4 {
        let rotation = Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch);
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

    pub fn on_pointer_down(&mut self, x: f32, y: f32) {
        self.dragging = true;
        self.last_x = x;
        self.last_y = y;
    }

    pub fn on_pointer_move(&mut self, x: f32, y: f32) {
        if !self.dragging {
            return;
        }

        let dx = (x - self.last_x) * 0.005;
        let dy = (y - self.last_y) * 0.005;

        self.yaw += dx;
        self.pitch += dy;

        // Normalize pitch to [-PI, PI] to keep it stable and prevent overflow
        self.pitch = (self.pitch + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;

        self.last_x = x;
        self.last_y = y;
    }

    pub fn on_pointer_up(&mut self, _x: f32, _y: f32) {
        self.dragging = false;
    }

    pub fn on_wheel(&mut self, delta_y: f32) {
        let zoom_factor = 1.0 + delta_y * 0.001;
        self.radius = (self.radius * zoom_factor).max(0.1);
    }
}
