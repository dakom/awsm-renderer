use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    gltf::{error::AwsmGltfError, pipelines::RenderPipelineKey, shaders::ShaderKey},
    mesh::{Mesh, MeshIndexBuffer, MeshVertexBuffer, PositionExtents},
    AwsmRenderer,
};
use awsm_renderer_core::{
    pipeline::primitive::{IndexFormat, PrimitiveTopology},
    shaders::ShaderModuleExt,
};
use glam::Vec3;

use super::{data::GltfData, layout::primitive_vertex_buffer_layout, pipelines::PipelineLayoutKey};

impl AwsmRenderer {
    pub async fn populate_gltf(
        &mut self,
        gltf_data: impl Into<Arc<GltfData>>,
        scene: Option<usize>,
    ) -> anyhow::Result<()> {
        let gltf_data = gltf_data.into();
        self.gltf.raw_datas.push(gltf_data.clone());

        let ctx = GltfPopulateContext { data: gltf_data };

        let scene = match scene {
            Some(index) => ctx
                .data
                .doc
                .scenes()
                .nth(index)
                .ok_or(AwsmGltfError::InvalidScene(index))?,
            None => ctx
                .data
                .doc
                .default_scene()
                .ok_or(AwsmGltfError::NoDefaultScene)?,
        };

        for node in scene.nodes() {
            self.populate_gltf_node(&ctx, &node, None).await?;
        }

        Ok(())
    }

    fn populate_gltf_node<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_node: &'b gltf::Node<'b>,
        _gltf_parent_node: Option<&'b gltf::Node<'b>>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>> {
        Box::pin(async move {
            if let Some(gltf_mesh) = gltf_node.mesh() {
                for gltf_primitive in gltf_mesh.primitives() {
                    self.populate_gltf_primitive(ctx, gltf_node, &gltf_mesh, gltf_primitive)
                        .await?;
                }
            }

            for child in gltf_node.children() {
                self.populate_gltf_node(ctx, &child, Some(gltf_node))
                    .await?;
            }
            Ok(())
        })
    }

    async fn populate_gltf_primitive(
        &mut self,
        ctx: &GltfPopulateContext,
        _gltf_node: &gltf::Node<'_>,
        gltf_mesh: &gltf::Mesh<'_>,
        gltf_primitive: gltf::Primitive<'_>,
    ) -> anyhow::Result<()> {
        let primitive_buffer_info =
            &ctx.data.buffers.meshes[gltf_mesh.index()][gltf_primitive.index()];

        let pipeline_layout_key = PipelineLayoutKey::new()
            .with_camera();

        let shader_key = ShaderKey::new(&gltf_primitive);

        let shader_module = match self.gltf.shaders.get(&shader_key) {
            None => {
                let shader_module = self.gpu.compile_shader(&shader_key.into_descriptor());
                shader_module.validate_shader().await?;

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

        let pipeline_key = RenderPipelineKey::new(self, shader_key, pipeline_layout_key, vec![vertex_buffer_layout]);

        let render_pipeline = match self.gltf.render_pipelines.get(&pipeline_key).cloned() {
            None => {
                let descriptor = pipeline_key.clone().into_descriptor(self, &shader_module)?;

                web_sys::console::log_1(&descriptor);

                let render_pipeline = self
                    .gpu
                    .create_render_pipeline(&descriptor)
                    .await?;

                self.gltf.render_pipelines.insert(pipeline_key, render_pipeline.clone());

                render_pipeline
            }
            Some(pipeline) => pipeline,
        };

        // TODO - transform nodes? lights? cameras? animations?

        let positions_attribute = gltf_primitive
            .attributes()
            .find_map(|(semantic, attribute)| {
                if semantic == gltf::Semantic::Positions {
                    Some(attribute)
                } else {
                    None
                }
            })
            .ok_or(AwsmGltfError::MissingPositionAttribute)?;

        let mut mesh = Mesh::new(
            render_pipeline,
            match gltf_primitive.indices() {
                Some(indices) => indices.count(),
                None => positions_attribute.count(),
            },
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
                offset: Some(primitive_buffer_info.vertex_offset as u64),
                size: Some((primitive_buffer_info.vertex_count * primitive_buffer_info.vertex_strides.iter().sum::<usize>()) as u64),
            }],
        )
        .with_topology(match gltf_primitive.mode() {
            gltf::mesh::Mode::Points => PrimitiveTopology::PointList,
            gltf::mesh::Mode::Lines => PrimitiveTopology::LineList,
            gltf::mesh::Mode::LineLoop => {
                return Err(AwsmGltfError::UnsupportedPrimitiveMode(gltf_primitive.mode()).into())
            }
            gltf::mesh::Mode::LineStrip => PrimitiveTopology::LineStrip,
            gltf::mesh::Mode::Triangles => PrimitiveTopology::TriangleList,
            gltf::mesh::Mode::TriangleStrip => PrimitiveTopology::TriangleStrip,
            gltf::mesh::Mode::TriangleFan => {
                return Err(AwsmGltfError::UnsupportedPrimitiveMode(gltf_primitive.mode()).into())
            }
        });

        if let Some(position_extents) = try_position_extents(&positions_attribute) {
            mesh = mesh.with_position_extents(position_extents);
        }

        if let Some(indices) = gltf_primitive.indices() {
            mesh = mesh.with_index_buffer(MeshIndexBuffer {
                // safe, only exists if we have an index
                buffer: ctx.data.buffers.index_buffer.clone().unwrap(),
                format: match indices.data_type() {
                    gltf::accessor::DataType::I16 => IndexFormat::Uint16,
                    gltf::accessor::DataType::U16 => IndexFormat::Uint16,
                    gltf::accessor::DataType::U32 => IndexFormat::Uint32,
                    _ => {
                        return Err(
                            AwsmGltfError::UnsupportedIndexDataType(indices.data_type()).into()
                        )
                    }
                },
                offset: primitive_buffer_info.index_offset.map(|x| x as u64),
                size: primitive_buffer_info.index_len.map(|x| x as u64),
            });
        }

        let _mesh_key = self.meshes.add(mesh);

        Ok(())
    }
}

fn try_position_extents(positions_attribute: &gltf::Accessor<'_>) -> Option<PositionExtents> {
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

    Some(PositionExtents {
        min: Vec3::new(min_x as f32, min_y as f32, min_z as f32),
        max: Vec3::new(max_x as f32, max_y as f32, max_z as f32),
    })
}

pub(super) struct GltfPopulateContext {
    pub data: Arc<GltfData>,
    // we may need more stuff here
}
