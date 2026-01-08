use awsm_renderer_core::command::render_pass::RenderPassEncoder;
use glam::Mat4;

use crate::{
    bounds::Aabb,
    error::AwsmError,
    mesh::{Mesh, MeshKey},
    pipelines::{compute_pipeline::ComputePipelineKey, render_pipeline::RenderPipelineKey},
    render::RenderContext,
    render_passes::geometry::bind_group::GeometryBindGroups,
    AwsmRenderer,
};

pub struct Renderables<'a> {
    pub opaque: Vec<Renderable<'a>>,
    pub transparent: Vec<Renderable<'a>>,
    pub hud: Vec<Renderable<'a>>,
}

impl AwsmRenderer {
    pub fn collect_renderables<'a>(&'a self, ctx: &RenderContext) -> Result<Renderables<'a>> {
        let _maybe_span_guard = if self.logging.render_timings {
            Some(tracing::span!(tracing::Level::INFO, "Collect renderables").entered())
        } else {
            None
        };

        let mut opaque = Vec::new();
        let mut transparent = Vec::new();
        let mut hud = Vec::new();

        for (mesh_key, mesh) in self.meshes.iter().filter(|(_k, m)| !m.hidden) {
            // TODO - frustum cull here
            if let Some(_world_aabb) = &mesh.world_aabb {
                // if !self.camera.frustum.intersects_aabb(&world_aabb) {
                //     continue; // skip meshes not in the camera frustum
                // }
            }

            let renderable = Renderable::Mesh {
                key: mesh_key,
                mesh,
                material_opaque_compute_pipeline_key: self
                    .render_passes
                    .material_opaque
                    .pipelines
                    // CHANGE to key too?
                    .get_compute_pipeline_key(mesh_key),
                material_transparent_render_pipeline_key: self
                    .render_passes
                    .material_transparent
                    .pipelines
                    .get_render_pipeline_key(mesh_key),
            };

            if mesh.hud {
                hud.push(renderable.clone());
            } else if self.materials.has_alpha_blend(mesh.material_key)
                || self.materials.has_alpha_mask(mesh.material_key)
            {
                transparent.push(renderable);
            } else {
                opaque.push(renderable);
            }
        }

        if let Some(camera_matrices) = self.camera.last_matrices.as_ref() {
            let view_proj = camera_matrices.view_projection();
            opaque.sort_by(|a, b| geometry_sort_renderable(ctx, a, b, &view_proj, false));
            transparent.sort_by(|a, b| geometry_sort_renderable(ctx, a, b, &view_proj, true));
            hud.sort_by(|a, b| geometry_sort_renderable(ctx, a, b, &view_proj, true));
        }

        Ok(Renderables {
            opaque,
            transparent,
            hud,
        })
    }
}

fn geometry_sort_renderable(
    ctx: &RenderContext,
    a: &Renderable,
    b: &Renderable,
    view_proj: &Mat4,
    transparent: bool,
) -> std::cmp::Ordering {
    // Criteria 2: group by render_pipeline_key.
    match (
        a.geometry_render_pipeline_key(ctx),
        b.geometry_render_pipeline_key(ctx),
    ) {
        (Err(_), Err(_)) => return std::cmp::Ordering::Equal,
        (Err(_), Ok(_)) => return std::cmp::Ordering::Greater,
        (Ok(_), Err(_)) => return std::cmp::Ordering::Less,
        (Ok(key_a), Ok(key_b)) => {
            let pipeline_ordering = key_a.cmp(&key_b);
            if pipeline_ordering != std::cmp::Ordering::Equal {
                return pipeline_ordering;
            }
        }
    }

    // Criteria 3: sort by depth.
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
                b_closest_depth.total_cmp(&a_closest_depth)
            } else {
                // Sort front-to-back for opaque objects.
                // (smaller z is closer, and we want that to come first)
                a_closest_depth.total_cmp(&b_closest_depth)
            }
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

#[derive(Debug, Clone)]
pub enum Renderable<'a> {
    Mesh {
        key: MeshKey,
        mesh: &'a Mesh,
        material_opaque_compute_pipeline_key: Option<ComputePipelineKey>,
        material_transparent_render_pipeline_key: Option<RenderPipelineKey>,
    },
}

impl Renderable<'_> {
    pub fn geometry_render_pipeline_key(&self, ctx: &RenderContext) -> Result<RenderPipelineKey> {
        match self {
            Self::Mesh { mesh, .. } => mesh.geometry_render_pipeline_key(ctx),
        }
    }

    pub fn material_opaque_compute_pipeline_key(&self) -> Option<ComputePipelineKey> {
        match self {
            Self::Mesh {
                material_opaque_compute_pipeline_key,
                ..
            } => *material_opaque_compute_pipeline_key,
        }
    }

    pub fn material_transparent_render_pipeline_key(
        &self,
        _ctx: &RenderContext,
    ) -> Option<RenderPipelineKey> {
        match self {
            Self::Mesh {
                material_transparent_render_pipeline_key,
                ..
            } => *material_transparent_render_pipeline_key,
        }
    }

    pub fn material_key(&self) -> crate::materials::MaterialKey {
        match self {
            Self::Mesh { mesh, .. } => mesh.material_key,
        }
    }

    pub fn world_aabb(&self) -> Option<&'_ Aabb> {
        match self {
            Self::Mesh { mesh, .. } => mesh.world_aabb.as_ref(),
        }
    }

    pub fn push_geometry_pass_commands(
        &self,
        ctx: &RenderContext,
        render_pass: &RenderPassEncoder,
        geometry_bind_groups: &GeometryBindGroups,
    ) -> Result<()> {
        match self {
            Self::Mesh { mesh, key, .. } => {
                mesh.push_geometry_pass_commands(ctx, *key, render_pass, geometry_bind_groups)
            }
        }
    }

    pub fn push_material_transparent_pass_commands(
        &self,
        ctx: &RenderContext,
        render_pass: &RenderPassEncoder,
        mesh_material_bind_group: &web_sys::GpuBindGroup,
    ) -> Result<()> {
        match self {
            Self::Mesh { mesh, key, .. } => mesh.push_material_transparent_pass_commands(
                ctx,
                *key,
                render_pass,
                mesh_material_bind_group,
            ),
        }
    }
}

type Result<T> = std::result::Result<T, AwsmError>;
