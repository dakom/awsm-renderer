mod orbit;
mod orthographic;
mod perspective;

use awsm_renderer::{camera::CameraExt, mesh::PositionExtents};
use glam::{Mat4, Quat, Vec2, Vec3};
use orbit::OrbitCamera;
use orthographic::OrthographicCamera;
use perspective::PerspectiveCamera;

#[derive(Debug, Clone)]
pub struct Camera {
    projection: CameraProjection,
    view: CameraView,
    bounding_radius: f32,
    margin: f32,
}

// This is what needs to be implemented to make the camera work with the renderer
impl CameraExt for Camera {
    fn projection_matrix(&self) -> Mat4 {
        match &self.projection {
            CameraProjection::Orthographic(camera) => camera.projection_matrix(),
            CameraProjection::Perspective(camera) => camera.projection_matrix(),
        }
    }

    fn view_matrix(&self) -> Mat4 {
        match &self.view {
            CameraView::Orbit(camera) => camera.get_view_matrix(),
        }
    }

    fn position_world(&self) -> Vec3 {
        match &self.view {
            CameraView::Orbit(camera) => camera.get_position(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CameraView {
    Orbit(OrbitCamera),
}

#[derive(Debug, Clone)]
pub enum CameraProjection {
    Orthographic(OrthographicCamera),
    Perspective(PerspectiveCamera),
}

impl Camera {
    pub fn new(canvas: web_sys::HtmlCanvasElement, extents: PositionExtents) -> Self {
        let center = (extents.min + extents.max) * 0.5;
        let size = extents.max - extents.min;

        let width = size.x;
        let height = size.y;
        let aspect = width / height;
        let mut half_w = width * 0.5;
        let mut half_h = height * 0.5;

        if half_w / half_h > aspect {
            half_h = half_w / aspect;
        } else {
            half_w = half_h * aspect;
        }

        let margin = 1.1;
        half_w *= margin;
        half_h *= margin;

        let bounding_radius = size.length() * 0.5;
        let radius = bounding_radius * margin;

        let view = OrbitCamera::new(center, radius);

        let mut camera = Self {
            projection: CameraProjection::Orthographic(OrthographicCamera {
                left: -half_w,
                right: half_w,
                bottom: -half_h,
                top: half_h,
                near: 0.01, // initial placeholder
                far: 100.0, // initial placeholder
            }),
            view: CameraView::Orbit(view),
            bounding_radius,
            margin,
        };

        camera.update_near_far();

        camera
    }

    /// Call this method whenever zoom changes to adjust clipping dynamically.
    pub fn update_near_far(&mut self) {
        if let (CameraProjection::Orthographic(ortho), CameraView::Orbit(orbit)) =
            (&mut self.projection, &self.view)
        {
            let bounding_radius = self.bounding_radius;
            let margin = self.margin;
            let camera_position = orbit.get_position();
            let distance = camera_position.distance(orbit.look_at);

            ortho.near = (distance - bounding_radius * margin * 2.0).max(0.01);
            ortho.far = distance + bounding_radius * margin * 2.0;
        }
    }

    pub fn on_pointer_down(&mut self, x: i32, y: i32) {
        match &mut self.view {
            CameraView::Orbit(orbit_view) => orbit_view.on_pointer_down(x as f32, y as f32),
        }
    }

    pub fn on_pointer_move(&mut self, x: i32, y: i32) {
        match &mut self.view {
            CameraView::Orbit(orbit_view) => orbit_view.on_pointer_move(x as f32, y as f32),
        }
    }

    pub fn on_pointer_up(&mut self, x: i32, y: i32) {
        match &mut self.view {
            CameraView::Orbit(orbit_view) => orbit_view.on_pointer_up(x as f32, y as f32),
        }
    }

    pub fn on_wheel(&mut self, delta: f64) {
        if let CameraView::Orbit(orbit_view) = &mut self.view {
            orbit_view.on_wheel(delta as f32);
        }

        if let CameraProjection::Orthographic(ortho) = &mut self.projection {
            ortho.zoom(1.0 + delta as f32 * 0.001);
        }

        // Update near/far after zooming
        self.update_near_far();
    }
}
