use std::sync::Arc;

use awsm_renderer_core::{buffer::{BufferDescriptor, BufferUsage}, shaders::ShaderModuleExt};

use crate::{
    gltf::{buffers::BufferKey, error::AwsmGltfError, pipelines::PipelineKey, shaders::ShaderKey}, mesh::Mesh, AwsmRenderer
};

use super::{cache::GltfResourceKey, loader::GltfResource};

impl AwsmRenderer {
    pub async fn populate_gltf(&mut self, gltf_res: GltfResource, scene: Option<usize>) -> anyhow::Result<()> {

        let gltf_res = Arc::new(gltf_res);
        let res_key = self.gltf.resources.len();
        self.gltf.resources.push(gltf_res.clone());

        let mut ctx = GltfPopulateContext {
            res_key,
            res: gltf_res
        };

        
        tracing::info!("Populating gltf resource...");

        for (i, gltf_buffer) in ctx.res.buffers.iter().enumerate() {
            let buffer = self.gpu.create_buffer(&BufferDescriptor::new(
                Some(&format!("gltf buffer #{i}")),
                gltf_buffer.len() as u64,
                BufferUsage::new()
                    .with_vertex()
            ).into())?;

            let buffer_key = BufferKey {
                gltf_res_key: res_key,
                index: i
            };

            self.gltf.buffers.insert(buffer_key, buffer);
        }

        let scene = match scene {
            Some(index) => ctx.clone().res.gltf.scenes().nth(index).ok_or(AwsmGltfError::InvalidScene(index))?,
            None => {
                ctx.clone().res.gltf.default_scene().ok_or(AwsmGltfError::NoDefaultScene)?
            }
        };


        for node in scene.nodes() {
            self.populate_gltf_node(&mut ctx, &node, None).await?;
        }

        Ok(())
    }


    pub async fn populate_gltf_node(&mut self, ctx: &mut GltfPopulateContext, gltf_node: &gltf::Node<'_>, gltf_parent_node: Option<&gltf::Node<'_>>) -> anyhow::Result<()> {
        if let Some(gltf_mesh) = gltf_node.mesh() {
            for gltf_primitive in gltf_mesh.primitives() {
                self.populate_gltf_primitive(ctx, &gltf_node, &gltf_mesh, &gltf_primitive).await?;
            }
        }

        for child in gltf_node.children() {
            self.populate_gltf_node(ctx, &child, Some(gltf_node)).await?;
        }
        Ok(())
    }

    pub async fn populate_gltf_primitive(&mut self, ctx: &mut GltfPopulateContext, gltf_node: &gltf::Node<'_>, gltf_mesh: &gltf::Mesh<'_>, gltf_primitive: &gltf::Primitive<'_>) -> anyhow::Result<()> {
        tracing::info!("Populating gltf primitive for node {}, mesh {}, primitive {}",
            gltf_node.index(),
            gltf_mesh.index(),
            gltf_primitive.index()
        );

        // TODO - actually load the data.. we should be able to get BufferKey from ctx and node info

        let shader_key = ShaderKey::default();

        let shader_module = match self.gltf.shaders.get(&shader_key) {
            None => {
                let shader_module = self.gpu.compile_shader(&shader_key.into_descriptor());
                shader_module.validate_shader().await?;

                // tracing::info!(
                //     "compiled shader: {:#?}",
                //     shader_module.get_compilation_info_ext().await?
                // );
                // tracing::info!("{}", shader_key.into_source());

                self.gltf
                    .shaders
                    .insert(shader_key.clone(), shader_module.clone());

                shader_module
            }
            Some(shader_module) => shader_module.clone(),
        };

        let mut pipeline_key = PipelineKey::new(self, shader_key);


        let pipeline = match self.gltf.pipelines.get(&pipeline_key) {
            None => {

                let pipeline = self
                    .gpu
                    .create_render_pipeline(&pipeline_key.clone().into_descriptor(&shader_module))
                    .await?;


                self.gltf
                    .pipelines
                    .insert(pipeline_key, pipeline.clone());

                pipeline
            }
            Some(pipeline) => pipeline.clone(),
        };

        // TODO - transform nodes? lights? cameras? animations?

        let mesh = Mesh::new(pipeline);
        let mesh_key = self.meshes.add(mesh);

        tracing::info!("Created mesh {mesh_key} from gltf");

        Ok(())
    }
}


#[derive(Clone)]
struct GltfPopulateContext {
    pub res: Arc<GltfResource>,
    pub res_key: GltfResourceKey
}