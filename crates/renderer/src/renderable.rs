use awsm_renderer_core::command::render_pass::RenderPassEncoder;
use glam::Mat4;

use crate::{
    bounds::Aabb, error::AwsmError, mesh::{Mesh, MeshKey}, pipelines::render_pipeline::RenderPipelineKey, render::RenderContext, render_passes::geometry::bind_group::GeometryBindGroups, transforms::TransformKey, AwsmRenderer
};

pub struct Renderables <'a> {
    pub opaque: Vec<Renderable<'a>>,
    pub transparent: Vec<Renderable<'a>>,
}

impl AwsmRenderer {
    pub fn collect_renderables(&self) -> Result<Renderables> {
        let _maybe_span_guard = if self.logging.render_timings {
            Some(tracing::span!(tracing::Level::INFO, "Collect renderables").entered())
        } else {
            None
        };

        let mut opaque = Vec::new();
        let mut transparent = Vec::new();

        for (key, mesh) in self.meshes.iter() {
            // TODO - frustum cull here
            if let Some(world_aabb) = &mesh.world_aabb {
                // if !self.camera.frustum.intersects_aabb(&world_aabb) {
                //     continue; // skip meshes not in the camera frustum
                // }
            }

            if self.materials.has_alpha_blend(mesh.material_key).unwrap_or(false) {
                transparent.push(Renderable::Mesh {
                    key,
                    mesh,
                });
            } else {
                opaque.push(Renderable::Mesh {
                    key,
                    mesh,
                });
            } 
        }

        if let Some(camera_matrices) = self.camera.last_matrices.as_ref() {
            let view_proj = camera_matrices.view_projection();
            opaque.sort_by(|a, b| sort_renderable(a, b, &view_proj, false));
            transparent.sort_by(|a, b| sort_renderable(a, b, &view_proj, true));
        }


        Ok(Renderables {
            opaque,
            transparent,
        })
    }

}

fn sort_renderable(a: &Renderable, b: &Renderable, view_proj: &Mat4, transparent: bool) -> std::cmp::Ordering {
    // Criteria 1: group by render_pipeline_key.
    let pipeline_ordering = a.render_pipeline_key().cmp(&b.render_pipeline_key());
    if pipeline_ordering != std::cmp::Ordering::Equal {
        return pipeline_ordering;
    }

    // Criterion 2: sort by depth.
    match (a.world_aabb(), b.world_aabb()) {
        (Some(a_world_aabb), Some(b_world_aabb)) => {
            let a_min_z = view_proj.transform_point3(a_world_aabb.min).z;
            let a_max_z = view_proj.transform_point3(a_world_aabb.max).z;

            let b_min_z = view_proj.transform_point3(b_world_aabb.min).z;
            let b_max_z = view_proj.transform_point3(b_world_aabb.max).z;

            let a_closest_depth = a_min_z.min(a_max_z);
            let b_closest_depth = b_min_z.min(b_max_z);

            if transparent {
                // Sort back-to-front for transparent objects.
                // (larger z is further away, and we want that to come first)
                b_closest_depth
                    .partial_cmp(&a_closest_depth)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                // Sort front-to-back for opaque objects.
                // (smaller z is closer, and we want that to come first) 
                a_closest_depth
                    .partial_cmp(&b_closest_depth)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        },
        _ => {
            // no AABBs, fallback to equality
            // TODO - maybe try to use the world matrix? Like:
            // w_axis is the translation vector in the world matrix
            // We use the z component for depth sorting.
            // let a_depth = a_world_mat.w_axis.z;
            // let b_depth = b_world_mat.w_axis.z;
            std::cmp::Ordering::Equal
        },
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

    pub fn world_aabb(&self) -> Option<&'_ Aabb> {
        match self {
            Self::Mesh { mesh, ..} => mesh.world_aabb.as_ref(), 
        }
    }

    pub fn push_geometry_pass_commands(&self, ctx: &RenderContext, render_pass: &RenderPassEncoder, geometry_bind_groups: &GeometryBindGroups) -> Result<()> {
        match self {
            Self::Mesh { mesh, key, .. } => mesh.push_geometry_pass_commands(ctx, *key, render_pass, geometry_bind_groups),
        }
    }
}

type Result<T> = std::result::Result<T, AwsmError>;
