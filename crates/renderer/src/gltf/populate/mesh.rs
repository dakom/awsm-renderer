use std::{future::Future, pin::Pin};

use crate::{
    bounds::Aabb,
    gltf::{
        error::{AwsmGltfError, Result},
        populate::material::GltfMaterialInfo,
    },
    materials::Material,
    mesh::{skins::SkinKey, Mesh, MeshBufferInfo, MeshBufferVertexInfo},
    pipeline_layouts::PipelineLayoutCacheKey,
    pipelines::render_pipeline::RenderPipelineCacheKey,
    transforms::{Transform, TransformKey},
    AwsmRenderer,
};
use awsm_renderer_core::{
    compare::CompareFunction,
    pipeline::{
        self,
        depth_stencil::DepthStencilState,
        fragment::{BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState},
        primitive::{CullMode, FrontFace, PrimitiveState, PrimitiveTopology},
    },
};
use glam::{Mat4, Vec3};

use super::GltfPopulateContext;

impl AwsmRenderer {
    pub(super) fn populate_gltf_node_mesh<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_node: &'b gltf::Node<'b>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            if let Some(gltf_mesh) = gltf_node.mesh() {
                // from the spec: "Only the joint transforms are applied to the skinned mesh; the transform of the skinned mesh node MUST be ignored."
                // so we swap out this node's transform with an identity matrix, but keep the hierarchy intact
                // might need to pass the joint transform key down too, not sure yet
                let mesh_transform_key = {
                    let node_to_transform = ctx.node_to_transform.lock().unwrap();
                    let transform_key = node_to_transform.get(&gltf_node.index()).cloned().unwrap();
                    if ctx
                        .transform_is_joint
                        .lock()
                        .unwrap()
                        .contains(&transform_key)
                    {
                        let parent_transform_key = self.transforms.get_parent(transform_key).ok();
                        self.transforms
                            .insert(Transform::IDENTITY, parent_transform_key)
                    } else {
                        transform_key
                    }
                };

                // We use the same matrices across the primitives
                // but the skin as a whole is defined on the mesh
                // from the spec: "When defined, mesh MUST also be defined."
                let mesh_skin_transform = ctx.node_to_skin_transform.lock().unwrap();
                let mesh_skin_transform = mesh_skin_transform.get(&gltf_node.index());

                for gltf_primitive in gltf_mesh.primitives() {
                    self.populate_gltf_primitive(
                        ctx,
                        gltf_node,
                        &gltf_mesh,
                        gltf_primitive,
                        mesh_transform_key,
                        mesh_skin_transform,
                    )
                    .await?;
                }
            }

            for child in gltf_node.children() {
                self.populate_gltf_node_mesh(ctx, &child).await?;
            }
            Ok(())
        })
    }

    async fn populate_gltf_primitive(
        &mut self,
        ctx: &GltfPopulateContext,
        gltf_node: &gltf::Node<'_>,
        gltf_mesh: &gltf::Mesh<'_>,
        gltf_primitive: gltf::Primitive<'_>,
        transform_key: TransformKey,
        skin_transform: Option<&(Vec<TransformKey>, Vec<Mat4>)>,
    ) -> Result<()> {
        let primitive_buffer_info =
            &ctx.data.buffers.meshes[gltf_mesh.index()][gltf_primitive.index()];

        let native_primitive_buffer_info = MeshBufferInfo::from(primitive_buffer_info.clone());

        let material_info =
            GltfMaterialInfo::new(self, ctx, primitive_buffer_info, gltf_primitive.material())
                .await?;

        let geometry_morph_key = match primitive_buffer_info.geometry_morph.clone() {
            None => None,
            Some(morph_buffer_info) => {
                let values = &ctx.data.buffers.geometry_morph_bytes;
                let values = &values[morph_buffer_info.values_offset
                    ..morph_buffer_info.values_offset + morph_buffer_info.values_size];

                // from spec: "The number of array elements MUST match the number of morph targets."
                // this is generally verified in the insert() call too
                let weights = gltf_mesh.weights().unwrap();
                let weights_u8 = unsafe {
                    std::slice::from_raw_parts(weights.as_ptr() as *const u8, (weights.len() * 4))
                };

                Some(self.meshes.morphs.geometry.insert_raw(
                    morph_buffer_info.into(),
                    weights_u8,
                    values,
                )?)
            }
        };

        // Material morphs are deprecated - all morphs (position, normal, tangent) are now in geometry_morph
        let material_morph_key = None;

        let skin_key = match (skin_transform, primitive_buffer_info.skin.clone()) {
            (None, None) => None,
            (Some(_), None) => {
                return Err(AwsmGltfError::SkinPartialData(
                    "Got transform but no buffers".to_string(),
                ));
            }
            (None, Some(_)) => {
                return Err(AwsmGltfError::SkinPartialData(
                    "Got buffers but no transform".to_string(),
                ));
            }
            (Some((joints, inverse_bind_matrices)), Some(info)) => {
                let index_weights = &ctx.data.buffers.skin_joint_index_weight_bytes;
                let index_weights = &index_weights[info.index_weights_offset
                    ..info.index_weights_offset + info.index_weights_size];
                Some(self.meshes.skins.insert(
                    joints.clone(),
                    inverse_bind_matrices,
                    info.set_count,
                    index_weights,
                )?)
            }
        };

        let material_key = self.materials.insert(
            Material::Pbr(material_info.material.clone()),
            &self.textures,
        );

        let geometry_render_pipeline_key = self
            .render_passes
            .geometry
            .pipelines
            .get_render_pipeline_key(
                material_info.material.double_sided(),
                ctx.transform_is_instanced
                    .lock()
                    .unwrap()
                    .contains(&transform_key),
            );

        let buffer_info_key = self
            .meshes
            .buffer_infos
            .insert(native_primitive_buffer_info);

        let mut mesh = Mesh::new(
            buffer_info_key,
            geometry_render_pipeline_key,
            transform_key,
            material_key,
        );

        if let Some(aabb) = try_position_aabb(&gltf_primitive) {
            mesh = mesh.with_aabb(aabb);
        }

        if let Some(morph_key) = geometry_morph_key {
            mesh = mesh.with_geometry_morph_key(morph_key);
        }

        if let Some(morph_key) = material_morph_key {
            mesh = mesh.with_material_morph_key(morph_key);
        }

        if let Some(skin_key) = skin_key {
            mesh = mesh.with_skin_key(skin_key);
        }

        let _mesh_key = {
            let visibility_data_start = primitive_buffer_info.vertex.offset;
            let visibility_data_end = visibility_data_start
                + MeshBufferVertexInfo::from(primitive_buffer_info.vertex.clone()).size();
            let visibility_data = &ctx.data.buffers.visibility_vertex_bytes
                [visibility_data_start..visibility_data_end];

            let attribute_data_start = primitive_buffer_info.triangles.vertex_attributes_offset;
            let attribute_data_end =
                attribute_data_start + primitive_buffer_info.triangles.vertex_attributes_size;
            let attribute_data =
                &ctx.data.buffers.attribute_vertex_bytes[attribute_data_start..attribute_data_end];

            let attribute_index_start = primitive_buffer_info
                .triangles
                .vertex_attribute_indices
                .offset;
            let attribute_index_end = attribute_index_start
                + primitive_buffer_info
                    .triangles
                    .vertex_attribute_indices
                    .total_size();
            let attribute_index =
                &ctx.data.buffers.index_bytes[attribute_index_start..attribute_index_end];

            self.meshes.insert(
                mesh,
                &self.materials,
                &self.transforms,
                visibility_data,
                attribute_data,
                attribute_index,
            )?
        };

        // Load all animations from the GLTF file
        // TODO: Add proper API for selecting/controlling which animations to play
        // Heuristic: Only load the first morph animation per node
        // This prevents multiple conflicting morph animations from playing simultaneously
        let mut has_morph_animation = false;

        for gltf_animation in ctx.data.doc.animations() {
            for channel in gltf_animation.channels() {
                if channel.target().node().index() == gltf_node.index() {
                    match channel.target().property() {
                        gltf::animation::Property::MorphTargetWeights => {
                            if has_morph_animation {
                                continue;
                            }
                            self.populate_gltf_animation_morph(
                                ctx,
                                gltf_animation
                                    .samplers()
                                    .nth(channel.sampler().index())
                                    .ok_or(AwsmGltfError::MissingAnimationSampler {
                                        animation_index: gltf_animation.index(),
                                        channel_index: channel.index(),
                                        sampler_index: channel.sampler().index(),
                                    })?,
                                geometry_morph_key,
                                material_morph_key,
                            )?;
                            has_morph_animation = true;
                        }
                        // transform animations were already populated in the node
                        gltf::animation::Property::Translation
                        | gltf::animation::Property::Rotation
                        | gltf::animation::Property::Scale => {}
                    }
                }
            }
        }

        Ok(())
    }
}

fn try_position_aabb(gltf_primitive: &gltf::Primitive<'_>) -> Option<Aabb> {
    let positions_attribute = gltf_primitive
        .attributes()
        .find_map(|(semantic, attribute)| {
            if semantic == gltf::Semantic::Positions {
                Some(attribute)
            } else {
                None
            }
        })?;

    let min = positions_attribute.min()?;
    let min = min.as_array()?;
    let max = positions_attribute.max()?;
    let max = max.as_array()?;

    if min.len() != 3 || max.len() != 3 {
        return None;
    }

    let min_x = min[0].as_f64()?;
    let min_y = min[1].as_f64()?;
    let min_z = min[2].as_f64()?;
    let max_x = max[0].as_f64()?;
    let max_y = max[1].as_f64()?;
    let max_z = max[2].as_f64()?;

    Some(Aabb {
        min: Vec3::new(min_x as f32, min_y as f32, min_z as f32),
        max: Vec3::new(max_x as f32, max_y as f32, max_z as f32),
    })
}
