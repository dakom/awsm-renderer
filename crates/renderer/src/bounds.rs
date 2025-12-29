use glam::{Mat4, Vec3};

// Axis-Aligned Bounding Box (AABB) structure
#[derive(Debug, Clone)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub const fn new_cube(width: f32, height: f32) -> Self {
        Self {
            min: Vec3::new(-width / 2.0, -height / 2.0, -width / 2.0),
            max: Vec3::new(width / 2.0, height / 2.0, width / 2.0),
        }
    }

    pub const fn new_unit_cube() -> Self {
        Self::new_cube(2.0, 2.0)
    }

    pub fn extend(&mut self, other: &Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    pub fn transform(&mut self, mat: &Mat4) {
        self.min = mat.transform_point3(self.min);
        self.max = mat.transform_point3(self.max);
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

#[cfg(feature = "gltf")]
impl Aabb {
    pub fn from_gltf_doc(doc: &gltf::Document) -> Self {
        let mut aabb: Option<Aabb> = None;

        // Helper function to recursively process nodes
        fn process_node(node: &gltf::Node, parent_transform: Mat4, aabb: &mut Option<Aabb>) {
            if let Some(mesh_aabb) = Aabb::from_gltf_node(node, Some(parent_transform)) {
                match aabb {
                    Some(ref mut existing) => existing.extend(&mesh_aabb),
                    None => *aabb = Some(mesh_aabb),
                }
            }

            let new_parent_transform =
                parent_transform * Mat4::from_cols_array_2d(&node.transform().matrix());
            // Recursively process child nodes
            for child in node.children() {
                process_node(&child, new_parent_transform, aabb);
            }
        }

        // Process all scenes in the document
        for scene in doc.scenes() {
            for node in scene.nodes() {
                process_node(&node, Mat4::IDENTITY, &mut aabb);
            }
        }

        // Return the calculated AABB or a default unit cube if no geometry found
        aabb.unwrap_or_else(Aabb::new_unit_cube)
    }

    pub fn from_gltf_node(node: &gltf::Node, parent_transform: Option<Mat4>) -> Option<Self> {
        let node_transform = match parent_transform {
            Some(transform) => transform * Mat4::from_cols_array_2d(&node.transform().matrix()),
            None => Mat4::from_cols_array_2d(&node.transform().matrix()),
        };

        let mut aabb: Option<Aabb> = None;

        // If this node has a mesh, process its bounds
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                if let Some(primitive_aabb) =
                    Aabb::from_gltf_primitive(&primitive, Some(node_transform))
                {
                    match aabb {
                        Some(ref mut existing) => existing.extend(&primitive_aabb),
                        None => aabb = Some(primitive_aabb),
                    }
                }
            }
        }

        aabb
    }

    pub fn from_gltf_primitive(
        primitive: &gltf::Primitive,
        transform: Option<Mat4>,
    ) -> Option<Self> {
        // Get position accessor to calculate bounds
        if let Some(position_accessor) = primitive.get(&gltf::Semantic::Positions) {
            if let (Some(min_val), Some(max_val)) =
                (position_accessor.min(), position_accessor.max())
            {
                if let (Some(min_arr), Some(max_arr)) = (min_val.as_array(), max_val.as_array()) {
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
                            if let Some(transform) = transform {
                                mesh_aabb.transform(&transform);
                            }

                            return Some(mesh_aabb);
                        }
                    }
                }
            }
        }

        None
    }
}
