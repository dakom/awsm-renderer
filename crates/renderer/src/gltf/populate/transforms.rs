//! glTF transform conversion helpers.

use glam::{Mat4, Quat, Vec3};

use crate::{
    transforms::{Transform, TransformKey},
    AwsmRenderer,
};

use super::GltfPopulateContext;
use crate::gltf::error::Result;

impl AwsmRenderer {
    pub(super) fn populate_gltf_node_transform<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_node: &'b gltf::Node<'b>,
        parent_transform_key: Option<TransformKey>,
    ) -> Result<()> {
        // We use one transform per-node, even though we are creating distinct meshes per gltf-primitive
        // conceptually, this means meshes (in the renderer) are more like components than individual nodes
        //
        // the reason is two-fold:
        // 1. that's technically how the gltf spec is defined
        // 2. we get a performance boost since we can use the same transform for all primitives in a mesh (instead of forcing an unnecessary tree)
        let transform = transform_gltf_node(gltf_node);
        let transform_key = self.transforms.insert(transform, parent_transform_key);

        ctx.key_lookups
            .lock()
            .unwrap()
            .insert_transform(gltf_node, transform_key);

        for child in gltf_node.children() {
            self.populate_gltf_node_transform(ctx, &child, Some(transform_key))?;
        }

        Ok(())
    }
}

/// Converts a glTF node transform into a renderer `Transform`.
pub fn transform_gltf_node(node: &gltf::Node) -> Transform {
    // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-node
    match node.transform() {
        gltf::scene::Transform::Matrix {
            matrix: gltf_matrix,
        } => {
            let matrix: Mat4 = Mat4::from_cols_array_2d(&gltf_matrix);
            Transform::from(matrix)
        }
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => Transform::from(
            glam::Mat4::from_translation(Vec3::from_array(translation))
                * glam::Mat4::from_quat(Quat::from_array(rotation))
                * glam::Mat4::from_scale(Vec3::from_array(scale)),
        ),
    }
}
