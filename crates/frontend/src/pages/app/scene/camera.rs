use awsm_renderer::{camera::CameraExt, mesh::PositionExtents};
use glam::{Mat4, Vec3};

#[derive(Debug, Clone)]
pub enum Camera {
    Orthographic(OrthographicCamera),
    Perspective(PerspectiveCamera),
}

impl Default for Camera {
    fn default() -> Self {
        Self::Orthographic(OrthographicCamera::default())
    }
}

impl Camera {
    pub fn set_extents(&mut self, extents: PositionExtents) {
        match self {
            Camera::Orthographic(camera) => camera.set_extents(extents.min, extents.max),
            Camera::Perspective(camera) => camera.set_extents(extents.min, extents.max),
        }
    }

    pub fn set_canvas(&mut self, canvas: &web_sys::HtmlCanvasElement) {
        match self {
            Camera::Orthographic(camera) => camera.set_canvas(canvas),
            Camera::Perspective(camera) => camera.set_canvas(canvas),
        }
    }
}

impl CameraExt for Camera {
    fn projection_matrix(&self) -> Mat4 {
        match self {
            Camera::Orthographic(camera) => camera.projection_matrix(),
            Camera::Perspective(camera) => camera.projection_matrix(),
        }
    }

    fn view_matrix(&self) -> Mat4 {
        match self {
            Camera::Orthographic(camera) => camera.view_matrix(),
            Camera::Perspective(camera) => camera.view_matrix(),
        }
    }

    fn position_world(&self) -> Vec3 {
        match self {
            Camera::Orthographic(camera) => camera.position,
            Camera::Perspective(camera) => camera.position,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
}

impl OrthographicCamera {
    pub fn projection_matrix(&self) -> Mat4 {
        // For WebGPU, we use orthographic_rh or orthographic_lh (not orthographic_rh_gl).
        Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        )
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    pub fn set_extents(&mut self, min: Vec3, max: Vec3) {
        // Calculate the left, right, bottom, and top extents based on the provided min and max
        let width = max.x - min.x;
        let height = max.y - min.y;

        let aspect_ratio = width / height;

        self.left = -min.x;
        self.right = max.x;
        self.bottom = -min.y;
        self.top = max.y;

        self.position = Vec3::new(0.0, 0.0, self.position.z);

        self.target = Vec3::new(self.position.x, self.position.y, self.target.z);

        self.up = Vec3::Y;

        let zoom_factor = 2.0;

        // Update the left, right, bottom, and top extents based on the zoom factor
        self.left *= zoom_factor;
        self.right *= zoom_factor;
        self.bottom *= zoom_factor;
        self.top *= zoom_factor;

        // but we need to keep the camera centered
        // erm, not sure if this is really right, but it seems to work so far...
        self.position.x = -(self.left + self.right) / 4.0;
        self.position.y = -(self.bottom + self.top) / 4.0;
        self.target.x = self.position.x;
        self.target.y = self.position.y;
    }

    pub fn set_canvas(&mut self, canvas: &web_sys::HtmlCanvasElement) {
        // TODO - should we even do this?
        // Set the camera's position based on the canvas size
        let width = canvas.client_width() as f32;
        let height = canvas.client_height() as f32;

        // Update the left, right, bottom, and top extents based on the canvas size
        self.left = -width / 2.0;
        self.right = width / 2.0;
        self.bottom = -height / 2.0;
        self.top = height / 2.0;

        // Update the projection matrix
        self.projection_matrix();
    }
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        Self {
            left: -10.0,
            right: 10.0,
            bottom: -10.0,
            top: 10.0,
            near: 0.1,
            far: 100.0,
            position: Vec3::new(0.0, 0.0, 10.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerspectiveCamera {
    pub fovy_radians: f32, // vertical field of view (in radians)
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
}

impl PerspectiveCamera {
    pub fn projection_matrix(&self) -> Mat4 {
        // For WebGPU, use perspective_rh or perspective_lh (NOT perspective_rh_gl).
        Mat4::perspective_rh(self.fovy_radians, self.aspect_ratio, self.near, self.far)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    pub fn set_extents(&mut self, min: Vec3, max: Vec3) {
        tracing::warn!("Perspective camera extents are not implemented yet");
    }

    pub fn set_canvas(&mut self, canvas: &web_sys::HtmlCanvasElement) {}
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self {
            fovy_radians: std::f32::consts::FRAC_PI_4, // 45 degrees
            aspect_ratio: 800.0 / 600.0,
            near: 0.1,
            far: 100.0,
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
        }
    }
}
