use std::{future::Future, pin::Pin};

use crate::{
    bounds::Aabb,
    gltf::{
        error::{AwsmGltfError, Result},
        layout::primitive_vertex_buffer_layout,
        pipelines::{PipelineLayoutKey, RenderPipelineKey},
    },
    mesh::{Mesh, MeshBufferInfo, MeshIndexBuffer, MeshVertexBuffer},
    shaders::{ShaderConstantIds, ShaderKey},
    transform::{Transform, TransformKey},
    AwsmRenderer,
};
use awsm_renderer_core::{
    pipeline::{
        fragment::ColorTargetState,
        primitive::{IndexFormat, PrimitiveTopology},
    },
    shaders::ShaderModuleExt,
};
use glam::Vec3;

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
                for gltf_primitive in gltf_mesh.primitives() {
                    self.populate_gltf_primitive(
                        ctx,
                        gltf_node,
                        &gltf_mesh,
                        gltf_primitive,
                        mesh_transform_key,
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
    ) -> Result<()> {
        let primitive_buffer_info =
            &ctx.data.buffers.meshes[gltf_mesh.index()][gltf_primitive.index()];

        let shader_key = ShaderKey::gltf_primitive_new(&gltf_primitive);

        let morph_key = match primitive_buffer_info.morph.clone() {
            None => None,
            Some(morph_buffer_info) => {
                // safe, can't have morph info without backing bytes
                let values = ctx.data.buffers.morph_bytes.as_ref().unwrap();
                let values = &values[morph_buffer_info.values_offset
                    ..morph_buffer_info.values_offset + morph_buffer_info.values_size];

                Some(
                    self.meshes
                        .morphs
                        .insert(morph_buffer_info.into(), values)?,
                )
            }
        };

        let pipeline_layout_key = PipelineLayoutKey::new(ctx, primitive_buffer_info);

        let shader_module = match self.gltf.shaders.get(&shader_key) {
            None => {
                let shader_module = self.gpu.compile_shader(&shader_key.into_descriptor());
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
        let vertex_buffer_layout =
            primitive_vertex_buffer_layout(&gltf_primitive, primitive_buffer_info)?;

        // tracing::info!("indices: {:?}", debug_slice_to_u16(ctx.data.buffers.index_bytes.as_ref().unwrap()));
        // tracing::info!("positions: {:?}", debug_slice_to_f32(&ctx.data.buffers.vertex_bytes[vertex_buffer_layout.attributes[0].offset as usize..]).chunks(3).take(3).collect::<Vec<_>>());
        //tracing::info!("normals: {:?}", debug_slice_to_f32(&ctx.data.buffers.vertex_bytes[vertex_buffer_layout.attributes[1].offset as usize..]).chunks(3).take(3).collect::<Vec<_>>());

        let mut pipeline_key = RenderPipelineKey::new(shader_key, pipeline_layout_key)
            .with_vertex_buffer_layout(vertex_buffer_layout)
            .with_fragment_target(ColorTargetState::new(self.gpu.current_context_format()));

        if let Some(morph) = &primitive_buffer_info.morph {
            pipeline_key = pipeline_key.with_vertex_constant(
                (ShaderConstantIds::MaxMorphTargets as u16).into(),
                (morph.targets_len as u32).into(),
            );
        }

        let render_pipeline = match self.gltf.render_pipelines.get(&pipeline_key).cloned() {
            None => {
                let descriptor =
                    pipeline_key
                        .clone()
                        .into_descriptor(self, &shader_module, morph_key)?;

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

        let primitive_buffer_info = MeshBufferInfo::from(primitive_buffer_info.clone());
        let mut mesh = Mesh::new(
            render_pipeline,
            primitive_buffer_info.draw_count(),
            transform_key,
        )
        .with_vertex_buffers(
            // We only need one vertex buffer per-mesh, because we've already constructed our buffers
            // to be one contiguous buffer of interleaved vertex data.
            vec![MeshVertexBuffer {
                buffer: ctx.data.buffers.vertex_buffer.clone(),
                // similar, but different, there is only one vertex layout (with multiple attributes)
                // slot here points to the first one
                slot: 0,
                // but we need to point to this primitive's slice within the larger buffer
                offset: Some(primitive_buffer_info.vertex.offset as u64),
                size: Some(primitive_buffer_info.vertex.size as u64),
            }],
        )
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
        });

        if let Some(aabb) = try_position_aabb(&gltf_primitive) {
            mesh = mesh.with_aabb(aabb);
        }

        if let Some(morph_key) = morph_key {
            mesh = mesh.with_morph_key(morph_key);
        }

        if let Some(indices) = gltf_primitive.indices() {
            mesh = mesh.with_index_buffer(MeshIndexBuffer {
                // safe, only exists if we have an index
                buffer: ctx.data.buffers.index_buffer.clone().unwrap(),
                format: match indices.data_type() {
                    gltf::accessor::DataType::I16 => IndexFormat::Uint16,
                    gltf::accessor::DataType::U16 => IndexFormat::Uint16,
                    gltf::accessor::DataType::U32 => IndexFormat::Uint32,
                    _ => return Err(AwsmGltfError::UnsupportedIndexDataType(indices.data_type())),
                },
                // these are safe, we for sure have an index buffer if we have indices
                offset: primitive_buffer_info.index.as_ref().unwrap().offset as u64,
                size: primitive_buffer_info.index_len().unwrap() as u64,
            });
        }

        let _mesh_key = self.meshes.insert(mesh);

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
