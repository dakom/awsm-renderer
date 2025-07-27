use std::{future::Future, pin::Pin};

use crate::{
    bounds::Aabb,
    gltf::{
        error::{AwsmGltfError, Result},
        layout::{instance_transform_vertex_buffer_layout, primitive_vertex_buffer_layout},
        populate::material::GltfMaterialInfo,
    },
    materials::Material,
    mesh::{Mesh, MeshBufferInfo},
    pipeline::{PipelineLayoutCacheKey, RenderPipelineCacheKey},
    shaders::{
        fragment::cache_key::ShaderCacheKeyFragment,
        vertex::{entry::mesh::ShaderCacheKeyVertexMesh, ShaderCacheKeyVertex},
        ShaderCacheKey,
    },
    skin::SkinKey,
    transform::{Transform, TransformKey},
    AwsmRenderer,
};
use awsm_renderer_core::{
    compare::CompareFunction,
    pipeline::{
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
                let mesh_skin_key = ctx
                    .node_to_skin
                    .lock()
                    .unwrap()
                    .get(&gltf_node.index())
                    .cloned();

                for gltf_primitive in gltf_mesh.primitives() {
                    self.populate_gltf_primitive(
                        ctx,
                        gltf_node,
                        &gltf_mesh,
                        gltf_primitive,
                        mesh_transform_key,
                        mesh_skin_key,
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
        skin_key: Option<SkinKey>,
    ) -> Result<()> {
        let primitive_buffer_info =
            &ctx.data.buffers.meshes[gltf_mesh.index()][gltf_primitive.index()];

        let material_info =
            GltfMaterialInfo::new(self, ctx, primitive_buffer_info, gltf_primitive.material())
                .await?;

        let shader_cache_key = ShaderCacheKey::new(
            ShaderCacheKeyVertex::Mesh(ShaderCacheKeyVertexMesh {
                attributes: primitive_buffer_info
                    .vertex
                    .attributes
                    .iter()
                    .map(|s| s.shader_key_kind)
                    .collect(),
                morphs: primitive_buffer_info
                    .morph
                    .as_ref()
                    .map(|m| m.shader_key)
                    .unwrap_or_default(),
                has_instance_transforms: ctx
                    .transform_is_instanced
                    .lock()
                    .unwrap()
                    .contains(&transform_key),
            }),
            ShaderCacheKeyFragment::Pbr(material_info.shader_cache_key),
        );

        let morph_key = match primitive_buffer_info.morph.clone() {
            None => None,
            Some(morph_buffer_info) => {
                // safe, can't have morph info without backing bytes
                let values = ctx.data.buffers.morph_bytes.as_ref().unwrap();
                let values = &values[morph_buffer_info.values_offset
                    ..morph_buffer_info.values_offset + morph_buffer_info.values_size];

                // from spec: "The number of array elements MUST match the number of morph targets."
                // this is generally verified in the insert() call too
                let weights = gltf_mesh.weights().unwrap();

                Some(
                    self.meshes
                        .morphs
                        .insert(morph_buffer_info.into(), weights, values)?,
                )
            }
        };

        let material_key = self
            .materials
            .insert(Material::Pbr(material_info.material.clone()));
        let material_bind_group_layout_key = self.add_material_pbr_bind_group_layout(
            material_key,
            &material_info.bind_group_layout_cache_key,
        )?;
        self.add_material_pbr_bind_group(
            material_key,
            material_bind_group_layout_key,
            &material_info.bind_group_cache_key,
        )?;

        let pipeline_layout_cache_key = PipelineLayoutCacheKey::new_mesh(
            material_bind_group_layout_key,
            morph_key.is_some(),
            skin_key.is_some(),
        );

        // we only need one vertex buffer per-mesh, because we've already constructed our buffers
        // to be one contiguous buffer of interleaved vertex data.
        // the attributes of this one vertex buffer layout contain all the info needed for the shader locations
        let (vertex_buffer_layout, shader_location) =
            primitive_vertex_buffer_layout(primitive_buffer_info)?;
        let instance_transform_vertex_buffer_layout =
            match shader_cache_key.vertex.as_mesh().has_instance_transforms {
                true => Some(instance_transform_vertex_buffer_layout(shader_location)),
                false => None,
            };

        // tracing::info!("indices: {:?}", debug_slice_to_u16(ctx.data.buffers.index_bytes.as_ref().unwrap()));
        // tracing::info!("positions: {:?}", debug_slice_to_f32(&ctx.data.buffers.vertex_bytes[vertex_buffer_layout.attributes[0].offset as usize..]).chunks(3).take(3).collect::<Vec<_>>());
        //tracing::info!("normals: {:?}", debug_slice_to_f32(&ctx.data.buffers.vertex_bytes[vertex_buffer_layout.attributes[1].offset as usize..]).chunks(3).take(3).collect::<Vec<_>>());

        let primitive_state = PrimitiveState::new()
            .with_topology(match gltf_primitive.mode() {
                gltf::mesh::Mode::Points => PrimitiveTopology::PointList,
                gltf::mesh::Mode::Lines => PrimitiveTopology::LineList,
                gltf::mesh::Mode::LineLoop => {
                    return Err(AwsmGltfError::UnsupportedPrimitiveMode(
                        gltf_primitive.mode(),
                    ))
                }
                gltf::mesh::Mode::LineStrip => PrimitiveTopology::LineStrip,
                gltf::mesh::Mode::Triangles => PrimitiveTopology::TriangleList,
                gltf::mesh::Mode::TriangleStrip => PrimitiveTopology::TriangleStrip,
                gltf::mesh::Mode::TriangleFan => {
                    return Err(AwsmGltfError::UnsupportedPrimitiveMode(
                        gltf_primitive.mode(),
                    ))
                }
            })
            .with_front_face(transform_to_winding_order(
                self.transforms
                    .get_world(transform_key)
                    .map_err(AwsmGltfError::TransformToWindingOrder)?,
            ))
            .with_cull_mode(match gltf_primitive.material().double_sided() {
                true => CullMode::None,
                false => CullMode::Back,
            });

        let mut color_target_state = ColorTargetState::new(self.renderable_texture_formats().scene);
        let mut depth_stencil_state =
            DepthStencilState::new(self.scene_target_depth_texture_format());
        // https://www.khronos.org/opengl/wiki/Blending#Blend_Equations
        if material_info.material.has_alpha_blend() {
            color_target_state.blend = Some(BlendState {
                color: BlendComponent::new()
                    .with_operation(BlendOperation::Add)
                    .with_src_factor(BlendFactor::SrcAlpha)
                    .with_dst_factor(BlendFactor::OneMinusSrcAlpha),
                alpha: BlendComponent::new()
                    .with_operation(BlendOperation::Add)
                    .with_src_factor(BlendFactor::One)
                    .with_dst_factor(BlendFactor::OneMinusSrcAlpha),
            });
            depth_stencil_state = depth_stencil_state
                .with_depth_write_enabled(false)
                .with_depth_compare(CompareFunction::LessEqual);
        } else {
            // This is also for cutoff materials, which are not alpha blended
            // but rather discarded if the alpha is below a threshold
            color_target_state.blend = None;
            depth_stencil_state = depth_stencil_state
                .with_depth_write_enabled(true)
                .with_depth_compare(CompareFunction::LessEqual);
        }

        let shader_key = self.add_shader(shader_cache_key.clone()).await?;

        let pipeline_layout_key =
            self.add_pipeline_layout(Some("gltf mesh primitive"), pipeline_layout_cache_key)?;

        let mut pipeline_cache_key = RenderPipelineCacheKey::new(shader_key, pipeline_layout_key)
            .with_primitive(primitive_state)
            .with_push_vertex_buffer_layout(vertex_buffer_layout)
            .with_push_fragment_target(color_target_state)
            .with_push_fragment_target(ColorTargetState::new(self.renderable_texture_formats().world_position))
            .with_depth_stencil(depth_stencil_state);

        if let Some(instance_transform_vertex_buffer_layout) =
            instance_transform_vertex_buffer_layout
        {
            pipeline_cache_key = pipeline_cache_key
                .with_push_vertex_buffer_layout(instance_transform_vertex_buffer_layout);
        }

        let render_pipeline_key = self
            .add_render_pipeline(Some("gltf mesh primitive"), pipeline_cache_key)
            .await?;

        let native_primitive_buffer_info = MeshBufferInfo::from(primitive_buffer_info.clone());
        let mut mesh = Mesh::new(
            render_pipeline_key,
            native_primitive_buffer_info.draw_count(),
            transform_key,
            material_key,
        );

        if let Some(aabb) = try_position_aabb(&gltf_primitive) {
            mesh = mesh.with_aabb(aabb);
        }

        if let Some(morph_key) = morph_key {
            mesh = mesh.with_morph_key(morph_key);
        }

        if let Some(skin_key) = skin_key {
            mesh = mesh.with_skin_key(skin_key);
        }

        let _mesh_key = {
            let index = match primitive_buffer_info.index.clone() {
                None => None,
                Some(index_buffer_info) => {
                    // safe, can't have info without backing bytes
                    let index_values = ctx.data.buffers.index_bytes.as_ref().unwrap();
                    let index_values = &index_values[index_buffer_info.offset
                        ..index_buffer_info.offset + index_buffer_info.total_size()];
                    Some((index_values, index_buffer_info.into()))
                }
            };

            let vertex_values = &ctx.data.buffers.vertex_bytes;
            let vertex_values = &vertex_values[primitive_buffer_info.vertex.offset
                ..primitive_buffer_info.vertex.offset + primitive_buffer_info.vertex.size];

            self.meshes.insert(
                mesh,
                vertex_values,
                native_primitive_buffer_info.vertex,
                index,
            )
        };

        for gltf_animation in ctx.data.doc.animations() {
            for channel in gltf_animation.channels() {
                if channel.target().node().index() == gltf_node.index() {
                    match channel.target().property() {
                        gltf::animation::Property::MorphTargetWeights => {
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
                                morph_key.ok_or(AwsmGltfError::MissingMorphForAnimation)?,
                            )?;
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

fn transform_to_winding_order(world_matrix: &Mat4) -> FrontFace {
    /*
     From spec: "When a mesh primitive uses any triangle-based topology (i.e., triangles, triangle strip, or triangle fan),
     the determinant of the node’s global transform defines the winding order of that primitive.
     If the determinant is a positive value, the winding order triangle faces is counterclockwise;
     in the opposite case, the winding order is clockwise.
    */
    if world_matrix.determinant() > 0.0 {
        FrontFace::Ccw
    } else {
        FrontFace::Cw
    }
}
