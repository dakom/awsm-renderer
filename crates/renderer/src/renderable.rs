use awsm_renderer_core::command::render_pass::RenderPassEncoder;

use crate::{
    error::AwsmError, mesh::{Mesh, MeshKey}, pipelines::render_pipeline::RenderPipelineKey, render::RenderContext, render_passes::geometry::bind_group::GeometryBindGroups, transforms::TransformKey, AwsmRenderer
};

impl AwsmRenderer {
    pub fn collect_renderables(&self, transparent: bool) -> Vec<Renderable<'_>> {
        let mut renderables = Vec::new();
        for (key, mesh) in self.meshes.iter() {
            let has_alpha = self.materials.has_alpha_blend(mesh.material_key).unwrap_or(false);
            if transparent && has_alpha || !transparent && !has_alpha {
                renderables.push(Renderable::Mesh {
                    key,
                    mesh,
                });
            } 
        }

        renderables.sort_by(|a, b| {
            // Criteria 1: group by render_pipeline_key.
            let pipeline_ordering = a.render_pipeline_key().cmp(&b.render_pipeline_key());
            if pipeline_ordering != std::cmp::Ordering::Equal {
                return pipeline_ordering;
            }

            // Criterion 2: sort by depth.
            match (a.transform_key(), b.transform_key()) {
                (Some(a_key), Some(b_key)) => {
                    let a_world_mat = self.transforms.get_world(a_key).unwrap();
                    let b_world_mat = self.transforms.get_world(b_key).unwrap();

                    // w_axis is the translation vector in the world matrix
                    // We use the z component for depth sorting.
                    let a_depth = a_world_mat.w_axis.z;
                    let b_depth = b_world_mat.w_axis.z;

                    if transparent {
                        // Sort back-to-front for transparent objects.
                        b_depth
                            .partial_cmp(&a_depth)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        // Sort front-to-back for opaque objects.
                        a_depth
                            .partial_cmp(&b_depth)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }
                }
                _ => std::cmp::Ordering::Equal,
            }
        });

        renderables
    }

}

pub enum Renderable<'a> {
    Mesh {
        key: MeshKey,
        mesh: &'a Mesh,
    },
}

impl Renderable<'_> {
    pub fn render_pipeline_key(&self) -> RenderPipelineKey {
        match self {
            Self::Mesh { mesh, .. } => mesh.render_pipeline_key,
        }
    }

    pub fn transform_key(&self) -> Option<TransformKey> {
        match self {
            Self::Mesh { mesh, .. } => Some(mesh.transform_key),
        }
    }

    pub fn push_geometry_pass_commands(&self, ctx: &RenderContext, render_pass: &RenderPassEncoder, geometry_bind_groups: &GeometryBindGroups) -> Result<()> {
        match self {
            Self::Mesh { mesh, key, .. } => mesh.push_geometry_pass_commands(ctx, *key, render_pass, geometry_bind_groups),
        }
    }
}

type Result<T> = std::result::Result<T, AwsmError>;
