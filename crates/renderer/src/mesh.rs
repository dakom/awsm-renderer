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
#[derive(Debug, Clone)]
pub struct Mesh {
    pub buffer_info_key: MeshBufferInfoKey,
    pub aabb: Option<Aabb>,
    pub world_aabb: Option<Aabb>, // this is the transformed AABB, used for frustum culling and depth sorting
    pub transform_key: TransformKey,
    pub material_key: MaterialKey,
    pub geometry_morph_key: Option<GeometryMorphKey>,
    pub material_morph_key: Option<MaterialMorphKey>,
    pub skin_key: Option<SkinKey>,
    pub double_sided: bool,
    pub instanced: bool,
}

impl Mesh {
    pub fn new(
        buffer_info_key: MeshBufferInfoKey,
        transform_key: TransformKey,
        material_key: MaterialKey,
        double_sided: bool,
        instanced: bool,
    ) -> Self {
        Self {
            buffer_info_key,
            transform_key,
            material_key,
            double_sided,
            instanced,
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

    pub fn geometry_render_pipeline_key(&self, ctx: &RenderContext) -> RenderPipelineKey {
        ctx.render_passes
            .geometry
            .pipelines
            .get_render_pipeline_key(self.double_sided, self.instanced, ctx.anti_aliasing)
    }

    pub fn push_geometry_pass_commands(
        &self,
        ctx: &RenderContext,
        mesh_key: MeshKey,
        render_pass: &RenderPassEncoder,
        bind_groups: &GeometryBindGroups,
    ) -> Result<()> {
        let meta_offset = ctx.meshes.meta.geometry_buffer_offset(mesh_key)? as u32;

        render_pass.set_bind_group(2, bind_groups.meta.get_bind_group()?, Some(&[meta_offset]))?;

        render_pass.set_vertex_buffer(
            0,
            ctx.meshes.visibility_geometry_data_gpu_buffer(),
            Some(
                ctx.meshes
                    .visibility_geometry_data_buffer_offset(mesh_key)? as u64,
            ),
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

        let buffer_info = ctx.meshes.buffer_infos.get(self.buffer_info_key)?;

        render_pass.set_index_buffer(
            ctx.meshes.visibility_geometry_index_gpu_buffer(),
            IndexFormat::Uint32,
            Some(
                ctx.meshes
                    .visibility_geometry_index_buffer_offset(mesh_key)? as u64,
            ),
            None,
        );

        let index_count = buffer_info.triangles.vertex_attribute_indices.count as u32;

        match ctx.instances.transform_instance_count(self.transform_key) {
            Some(instance_count) => {
                render_pass.draw_indexed_with_instance_count(index_count, instance_count as u32);
            }
            _ => {
                render_pass.draw_indexed(index_count);
            }
        }

        Ok(())
    }

    pub fn push_material_transparent_pass_commands(
        &self,
        ctx: &RenderContext,
        mesh_key: MeshKey,
        render_pass: &RenderPassEncoder,
        mesh_material_bind_group: &web_sys::GpuBindGroup,
    ) -> Result<()> {
        let geometry_meta_offset = ctx.meshes.meta.geometry_buffer_offset(mesh_key)? as u32;
        let material_meta_offset = ctx.meshes.meta.material_buffer_offset(mesh_key)? as u32;
        let buffer_info = ctx.meshes.buffer_infos.get(self.buffer_info_key)?;

        render_pass.set_bind_group(
            3,
            mesh_material_bind_group,
            Some(&[geometry_meta_offset, material_meta_offset]),
        )?;

        // Geometry stuff Slot 0 (locations 0-4)
        render_pass.set_vertex_buffer(
            0,
            ctx.meshes.transparency_geometry_data_gpu_buffer(),
            Some(
                ctx.meshes
                    .transparency_geometry_data_buffer_offset(mesh_key)? as u64,
            ),
            None,
        );

        // Instancing Slot 1 (locations 5-8)
        let attribute_slot =
            if let Ok(offset) = ctx.instances.transform_buffer_offset(self.transform_key) {
                render_pass.set_vertex_buffer(
                    1,
                    ctx.instances.gpu_transform_buffer(),
                    Some(offset as u64),
                    None,
                );

                2
            } else {
                1
            };

        // Attributes
        // If instanced: slot 2 (locations 9+)
        // If not instanced: slot 1 (locations 5+)
        render_pass.set_vertex_buffer(
            attribute_slot,
            ctx.meshes.custom_attribute_data_gpu_buffer(),
            Some(ctx.meshes.custom_attribute_data_buffer_offset(mesh_key)? as u64),
            None,
        );

        render_pass.set_index_buffer(
            ctx.meshes.transparency_geometry_index_gpu_buffer(),
            IndexFormat::Uint32,
            Some(
                ctx.meshes
                    .transparency_geometry_index_buffer_offset(mesh_key)? as u64,
            ),
            None,
        );

        let index_count = buffer_info.triangles.vertex_attribute_indices.count as u32;

        match ctx.instances.transform_instance_count(self.transform_key) {
            Some(instance_count) => {
                render_pass.draw_indexed_with_instance_count(index_count, instance_count as u32);
            }
            _ => {
                render_pass.draw_indexed(index_count);
            }
        }

        Ok(())
    }
}
