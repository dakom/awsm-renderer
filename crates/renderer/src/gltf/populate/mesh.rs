use std::{future::Future, pin::Pin};

use crate::{
    bounds::Aabb,
    gltf::{
        error::{AwsmGltfError, Result},
        layout::primitive_vertex_buffer_layout,
        pipelines::{PipelineLayoutKey, RenderPipelineKey},
    },
    mesh::{Mesh, MeshBufferInfo},
    shaders::{ShaderConstantIds, ShaderKey},
    skin::SkinKey,
    transform::{Transform, TransformKey},
    AwsmRenderer,
};
use awsm_renderer_core::{
    pipeline::{fragment::ColorTargetState, primitive::{CullMode, FrontFace, PrimitiveState, PrimitiveTopology}},
    shaders::ShaderModuleExt,
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

        let shader_key = ShaderKey::new(
            primitive_buffer_info
                .vertex
                .attributes
                .iter()
                .map(|s| s.shader_key_kind)
                .collect(),
            primitive_buffer_info.morph.as_ref().map(|m| m.shader_key),
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

        let pipeline_layout_key = PipelineLayoutKey::new(ctx, primitive_buffer_info);

        let shader_module = match self.gltf.shaders.get(&shader_key) {
            None => {
                let shader_module = self.gpu.compile_shader(&shader_key.into_descriptor()?);
                shader_module
                    .validate_shader()
                    .await
                    .map_err(AwsmGltfError::MeshPrimitiveShader)?;

                self.gltf
                    .shaders
                    .insert(shader_key.clone(), shader_module.clone());

                shader_module
            }
            Some(shader_module) => shader_module.clone(),
        };

        // we only need one vertex buffer per-mesh, because we've already constructed our buffers
        // to be one contiguous buffer of interleaved vertex data.
        // the attributes of this one vertex buffer layout contain all the info needed for the shader locations
        let vertex_buffer_layout = primitive_vertex_buffer_layout(primitive_buffer_info)?;

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
            .with_front_face(transform_to_winding_order(self.transforms.get_world(transform_key).map_err(AwsmGltfError::TransformToWindingOrder)?))
            .with_cull_mode(match gltf_primitive.material().double_sided() {
                true => CullMode::None,
                false => CullMode::Back, 
            });

        let mut pipeline_key = RenderPipelineKey::new(shader_key, pipeline_layout_key)
            .with_primitive(primitive_state)
            .with_vertex_buffer_layout(vertex_buffer_layout)
            .with_fragment_target(ColorTargetState::new(self.gpu.current_context_format()));

        if let Some(morph) = &primitive_buffer_info.morph {
            pipeline_key = pipeline_key.with_vertex_constant(
                (ShaderConstantIds::MorphTargetLen as u16).into(),
                (morph.targets_len as u32).into(),
            );
        }

        let render_pipeline = match self.gltf.render_pipelines.get(&pipeline_key).cloned() {
            None => {
                let descriptor = pipeline_key.clone().into_descriptor(
                    self,
                    &shader_module,
                    morph_key,
                    skin_key,
                )?;

                let render_pipeline = self
                    .gpu
                    .create_render_pipeline(&descriptor)
                    .await
                    .map_err(AwsmGltfError::MeshPrimitiveRenderPipeline)?;

                self.gltf
                    .render_pipelines
                    .insert(pipeline_key, render_pipeline.clone());

                render_pipeline
            }
            Some(pipeline) => pipeline,
        };

        let native_primitive_buffer_info = MeshBufferInfo::from(primitive_buffer_info.clone());
        let mut mesh = Mesh::new(
            render_pipeline,
            native_primitive_buffer_info.draw_count(),
            transform_key,
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