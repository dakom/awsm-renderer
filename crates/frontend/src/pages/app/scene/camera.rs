mod projection;
mod view;

use awsm_renderer::bounds::Aabb;
use awsm_renderer::camera::CameraMatrices;
use glam::{Mat4, Vec3};
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
impl Camera {
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

    pub fn matrices(&self) -> CameraMatrices {
        CameraMatrices {
            view: self.view_matrix(),
            projection: self.projection_matrix(),
            position_world: self.position_world(),
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
    pub fn new_orthographic(
        aabb: Option<Aabb>,
        gltf_doc: Option<gltf::Document>,
        aspect: f32,
    ) -> Self {
        let margin = 1.1;

        let aabb = aabb.unwrap_or_else(|| {
            if let Some(doc) = &gltf_doc {
                aabb_from_gltf_doc(doc)
            } else {
                aabb_unit_cube()
            }
        });

        let mut view = OrbitCamera::new_aabb(&aabb, margin);
        if let Some(doc) = &gltf_doc {
            view.setup_from_gltf(doc);
        }
        let view = CameraView::Orbit(view);

        let mut projection = OrthographicCamera::new_aabb(&view, &aabb, margin, aspect);
        if let Some(doc) = &gltf_doc {
            projection.setup_from_gltf(doc);
        }
        let projection = CameraProjection::Orthographic(projection);

        Self {
            projection,
            view,
            aabb,
            margin,
        }
    }

    pub fn new_perspective(
        aabb: Option<Aabb>,
        gltf_doc: Option<gltf::Document>,
        aspect: f32,
    ) -> Self {
        let margin = 1.1;
        let aabb = aabb.unwrap_or_else(|| {
            if let Some(doc) = &gltf_doc {
                aabb_from_gltf_doc(doc)
            } else {
                aabb_unit_cube()
            }
        });

        let mut view = OrbitCamera::new_aabb(&aabb, margin);
        if let Some(doc) = &gltf_doc {
            view.setup_from_gltf(doc);
        }
        let view = CameraView::Orbit(view);

        let mut projection = PerspectiveCamera::new_aabb(&view, &aabb, margin, aspect);
        if let Some(doc) = &gltf_doc {
            projection.setup_from_gltf(doc);
        }
        let projection = CameraProjection::Perspective(projection);

        Self {
            projection,
            view,
            aabb,
            margin,
        }
    }

    pub fn on_pointer_down(&mut self) {
        match &mut self.view {
            CameraView::Orbit(orbit_view) => orbit_view.on_pointer_down(),
        }
    }

    pub fn on_pointer_move(&mut self, x: i32, y: i32) {
        match &mut self.view {
            CameraView::Orbit(orbit_view) => orbit_view.on_pointer_move(x as f32, y as f32),
        }
    }

    pub fn on_pointer_up(&mut self) {
        match &mut self.view {
            CameraView::Orbit(orbit_view) => orbit_view.on_pointer_up(),
        }
    }

    pub fn on_wheel(&mut self, delta: f64) {
        match &mut self.view {
            CameraView::Orbit(orbit_view) => orbit_view.on_wheel(delta as f32),
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

fn aabb_from_gltf_doc(doc: &gltf::Document) -> Aabb {
    let mut aabb: Option<Aabb> = None;

    // Helper function to recursively process nodes
    fn process_node(node: &gltf::Node, transform: Mat4, aabb: &mut Option<Aabb>) {
        let node_transform = transform * Mat4::from_cols_array_2d(&node.transform().matrix());

        // If this node has a mesh, process its bounds
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                // Get position accessor to calculate bounds
                if let Some(position_accessor) = primitive.get(&gltf::Semantic::Positions) {
                    if let (Some(min_val), Some(max_val)) =
                        (position_accessor.min(), position_accessor.max())
                    {
                        if let (Some(min_arr), Some(max_arr)) =
                            (min_val.as_array(), max_val.as_array())
                        {
                            if min_arr.len() == 3 && max_arr.len() == 3 {
                                if let (
                                    Some(min_x),
                                    Some(min_y),
                                    Some(min_z),
                                    Some(max_x),
                                    Some(max_y),
                                    Some(max_z),
                                ) = (
                                    min_arr[0].as_f64(),
                                    min_arr[1].as_f64(),
                                    min_arr[2].as_f64(),
                                    max_arr[0].as_f64(),
                                    max_arr[1].as_f64(),
                                    max_arr[2].as_f64(),
                                ) {
                                    let min = Vec3::new(min_x as f32, min_y as f32, min_z as f32);
                                    let max = Vec3::new(max_x as f32, max_y as f32, max_z as f32);

                                    let mut mesh_aabb = Aabb::new(min, max);
                                    mesh_aabb.transform(&node_transform);

                                    match aabb {
                                        Some(ref mut existing) => existing.extend(&mesh_aabb),
                                        None => *aabb = Some(mesh_aabb),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Recursively process child nodes
        for child in node.children() {
            process_node(&child, node_transform, aabb);
        }
    }

    // Process all scenes in the document
    for scene in doc.scenes() {
        for node in scene.nodes() {
            process_node(&node, Mat4::IDENTITY, &mut aabb);
        }
    }

    // Return the calculated AABB or a default unit cube if no geometry found
    aabb.unwrap_or_else(aabb_unit_cube)
}

fn aabb_unit_cube() -> Aabb {
    Aabb {
        min: Vec3::new(-1.0, -1.0, -1.0),
        max: Vec3::new(1.0, 1.0, 1.0),
    }
}
