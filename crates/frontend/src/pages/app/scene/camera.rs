mod projection;
mod view;

use awsm_renderer::bounds::Aabb;
use awsm_renderer::camera::CameraExt;
use glam::{Mat4, Quat, Vec2, Vec3};
use projection::orthographic::OrthographicCamera;
use projection::perspective::PerspectiveCamera;
use view::orbit::OrbitCamera;

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum CameraId {
    #[default]
    Orthographic,
    Perspective,
}

#[derive(Debug, Clone)]
pub struct Camera {
    projection: CameraProjection,
    view: CameraView,
    aabb: Aabb,
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

impl CameraView {
    pub fn position(&self) -> Vec3 {
        match self {
            CameraView::Orbit(camera) => camera.get_position(),
        }
    }

    pub fn look_at(&self) -> Vec3 {
        match self {
            CameraView::Orbit(camera) => camera.look_at,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CameraProjection {
    Orthographic(OrthographicCamera),
    Perspective(PerspectiveCamera),
}

impl Camera {
    pub fn new_orthographic(aabb: Aabb, aspect: f32) -> Self {
        let margin = 1.1;
        let view = CameraView::Orbit(OrbitCamera::new_aabb(&aabb, margin));
        let projection = CameraProjection::Orthographic(OrthographicCamera::new_aabb(
            &view, &aabb, margin, aspect,
        ));

        Self {
            projection,
            view,
            aabb,
            margin,
        }
    }

    pub fn new_perspective(aabb: Aabb, aspect: f32) -> Self {
        let margin = 1.1;
        let view = CameraView::Orbit(OrbitCamera::new_aabb(&aabb, margin));
        let projection = CameraProjection::Perspective(PerspectiveCamera::new_aabb(
            &view, &aabb, margin, aspect,
        ));

        Self {
            projection,
            view,
            aabb,
            margin,
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

        match &mut self.projection {
            CameraProjection::Orthographic(ortho) => {
                ortho.on_wheel(&self.view, &self.aabb, self.margin, delta as f32);
            }
            CameraProjection::Perspective(persp) => {
                persp.on_wheel(&self.view, &self.aabb, self.margin);
            }
        }
    }

    pub fn on_resize(&mut self, aspect: f32) {
        match &mut self.projection {
            CameraProjection::Orthographic(ortho) => {
                ortho.on_resize(&self.view, &self.aabb, self.margin, aspect);
            }
            CameraProjection::Perspective(persp) => {
                persp.on_resize(aspect);
            }
        }
    }
}
