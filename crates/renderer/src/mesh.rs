mod buffer_info;
mod error;
mod meshes;
pub mod meta;
pub mod morphs;
pub mod skins;

use awsm_renderer_core::{
    command::render_pass::RenderPassEncoder, pipeline::primitive::IndexFormat,
};

use crate::materials::MaterialKey;
use crate::mesh::morphs::{GeometryMorphKey, MaterialMorphKey};
use crate::render::RenderContext;
use crate::render_passes::geometry::bind_group::GeometryBindGroups;
use crate::transforms::TransformKey;
use crate::{bounds::Aabb, pipelines::render_pipeline::RenderPipelineKey};
use skins::SkinKey;

pub use buffer_info::*;
pub use error::AwsmMeshError;
pub use meshes::{MeshKey, Meshes};

use super::error::Result;

// this is most like a "primitive" in gltf, not the containing "mesh"
// because for non-gltf naming, "mesh" makes more sense
#[derive(Debug)]
pub struct Mesh {
    pub render_pipeline_key: RenderPipelineKey,
    pub aabb: Option<Aabb>,
    pub world_aabb: Option<Aabb>, // this is the transformed AABB, used for frustum culling and depth sorting
    pub transform_key: TransformKey,
    pub material_key: MaterialKey,
    pub geometry_morph_key: Option<GeometryMorphKey>,
    pub material_morph_key: Option<MaterialMorphKey>,
    pub skin_key: Option<SkinKey>,
}

impl Mesh {
    pub fn new(
        render_pipeline_key: RenderPipelineKey,
        transform_key: TransformKey,
        material_key: MaterialKey,
    ) -> Self {
        Self {
            render_pipeline_key,
            transform_key,
            material_key,
            aabb: None,
            world_aabb: None,
            geometry_morph_key: None,
            material_morph_key: None,
            skin_key: None,
        }
    }

    pub fn with_aabb(mut self, aabb: Aabb) -> Self {
        self.aabb = Some(aabb.clone());
        self.world_aabb = Some(aabb); // initially, world_aabb is the same as aabb
        self
    }

    pub fn with_geometry_morph_key(mut self, morph_key: GeometryMorphKey) -> Self {
        self.geometry_morph_key = Some(morph_key);
        self
    }

    pub fn with_material_morph_key(mut self, morph_key: MaterialMorphKey) -> Self {
        self.material_morph_key = Some(morph_key);
        self
    }

    pub fn with_skin_key(mut self, skin_key: SkinKey) -> Self {
        self.skin_key = Some(skin_key);
        self
    }

    pub fn push_geometry_pass_commands(
        &self,
        ctx: &RenderContext,
        mesh_key: MeshKey,
        render_pass: &RenderPassEncoder,
        geometry_bind_groups: &GeometryBindGroups,
    ) -> Result<()> {

        // if _any_ shapes are used, set the bind group
        // unused shapes will simply be ignored (so 0 offset is fine)
        let (morph_weights_offset, morph_values_offset) = match self.geometry_morph_key {
            Some(morph_key) => (
                ctx.meshes
                    .morphs
                    .geometry
                    .weights_buffer_offset(morph_key)? as u32,
                ctx.meshes.morphs.geometry.values_buffer_offset(morph_key)? as u32,
            ),
            None => (0, 0),
        };

        let (skin_matrices_offset, skin_index_weights_offset) = match self.skin_key {
            Some(skin_key) => (
                ctx.meshes.skins.joint_matrices_offset(skin_key)? as u32,
                ctx.meshes.skins.joint_index_weights_offset(skin_key)? as u32,
            ),
            None => (0, 0),
        };

        let meta_offset = ctx.meshes.meta_data_buffer_offset(mesh_key)? as u32;

        render_pass.set_bind_group(
            2,
            geometry_bind_groups.meta.get_bind_group()?,
            Some(&[meta_offset]),
        )?;

        render_pass.set_bind_group(
            3,
            geometry_bind_groups.animation.get_bind_group()?,
            Some(&[
                morph_weights_offset,
                morph_values_offset,
                skin_matrices_offset,
                skin_index_weights_offset,
            ]),
        )?;

        render_pass.set_vertex_buffer(
            0,
            ctx.meshes.visibility_data_gpu_buffer(),
            Some(ctx.meshes.visibility_data_buffer_offset(mesh_key)? as u64),
            None,
        );

        if let Ok(offset) = ctx.instances.transform_buffer_offset(self.transform_key) {
            render_pass.set_vertex_buffer(
                1,
                ctx.instances.gpu_transform_buffer(),
                Some(offset as u64),
                None,
            );
        }

        let buffer_info = ctx.meshes.buffer_info(mesh_key)?;

        render_pass.set_index_buffer(
            ctx.meshes.visibility_index_gpu_buffer(),
            IndexFormat::Uint32,
            Some(ctx.meshes.visibility_index_buffer_offset(mesh_key)? as u64),
            None,
        );

        match ctx.instances.transform_instance_count(self.transform_key) {
            Some(instance_count) => {
                render_pass.draw_indexed_with_instance_count(
                    buffer_info.vertex.count as u32,
                    instance_count as u32,
                );
            }
            _ => {
                render_pass.draw_indexed(buffer_info.vertex.count as u32);
            }
        }

        Ok(())
    }
}
