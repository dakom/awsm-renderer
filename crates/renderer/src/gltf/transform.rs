use glam::{Mat4, Quat, Vec3};

use crate::transform::Transform;

pub(super) fn transform_gltf_node(node: &gltf::Node) -> Transform {
    // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-node
    match node.transform() {
        gltf::scene::Transform::Matrix {
            matrix: gltf_matrix,
        } => {
            let matrix: Mat4 = Mat4::from_cols_array_2d(&gltf_matrix);
            Transform::from_matrix(matrix)
        }
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => Transform::from_matrix(
            glam::Mat4::from_translation(Vec3::from_array(translation))
                * glam::Mat4::from_quat(Quat::from_array(rotation))
                * glam::Mat4::from_scale(Vec3::from_array(scale)),
        ),
    }
}
