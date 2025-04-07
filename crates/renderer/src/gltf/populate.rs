use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    gltf::{error::AwsmGltfError, pipelines::PipelineKey, shaders::ShaderKey},
    mesh::{Mesh, MeshIndexBuffer, MeshVertexBuffer},
    AwsmRenderer,
};
use awsm_renderer_core::{
    pipeline::primitive::{IndexFormat, PrimitiveTopology},
    shaders::ShaderModuleExt,
};

use super::{data::GltfData, layout::primitive_vertex_buffer_layout};

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
        let mesh_primitive_offset =
            &ctx.data.buffers.meshes[gltf_mesh.index()][gltf_primitive.index()];

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
            primitive_vertex_buffer_layout(&gltf_primitive, mesh_primitive_offset)?;

        let pipeline_key = PipelineKey::new(self, shader_key, vec![vertex_buffer_layout]);

        let pipeline = match self.gltf.pipelines.get(&pipeline_key) {
            None => {
                let pipeline = self
                    .gpu
                    .create_render_pipeline(
                        &pipeline_key.clone().into_descriptor(self, &shader_module)?,
                    )
                    .await?;

                self.gltf.pipelines.insert(pipeline_key, pipeline.clone());

                pipeline
            }
            Some(pipeline) => pipeline.clone(),
        };

        // TODO - transform nodes? lights? cameras? animations?

        let mut mesh = Mesh::new(
            pipeline,
            match gltf_primitive.indices() {
                Some(indices) => indices.count(),
                None => gltf_primitive
                    .attributes()
                    .find_map(|(semantic, attribute)| {
                        if semantic == gltf::Semantic::Positions {
                            Some(attribute.count())
                        } else {
                            None
                        }
                    })
                    .ok_or(AwsmGltfError::MissingPositionAttribute)?,
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
                offset: Some(mesh_primitive_offset.vertex as u64),
                size: Some(mesh_primitive_offset.total_vertex_len() as u64),
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
        //.with_position_extents();

        if let Some(index) = mesh_primitive_offset.index {
            mesh = mesh.with_index_buffer(MeshIndexBuffer {
                // safe, only exists if we have an index
                buffer: ctx.data.buffers.index_buffer.clone().unwrap(),
                format: match gltf_primitive.indices().unwrap().data_type() {
                    gltf::accessor::DataType::I16 => IndexFormat::Uint16,
                    gltf::accessor::DataType::U16 => IndexFormat::Uint16,
                    gltf::accessor::DataType::U32 => IndexFormat::Uint32,
                    _ => {
                        return Err(AwsmGltfError::UnsupportedIndexDataType(
                            gltf_primitive.indices().unwrap().data_type(),
                        )
                        .into())
                    }
                },
                offset: Some(index as u64),
                size: mesh_primitive_offset.index_len.map(|x| x as u64),
            });
        }

        let _mesh_key = self.meshes.add(mesh);

        Ok(())
    }
}

pub(super) struct GltfPopulateContext {
    pub data: Arc<GltfData>,
    // we may need more stuff here
}
