use awsm_renderer_core::shaders::ShaderModuleExt;

use crate::{
    gltf::{pipelines::PipelineKey, shaders::ShaderKey}, mesh::Mesh, AwsmRenderer
};

use super::loader::GltfResource;

impl AwsmRenderer {
    pub async fn populate_gltf(&mut self, _gltf_res: &GltfResource) -> anyhow::Result<()> {
        tracing::info!("Populating gltf resource...");

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

        let pipeline_key = PipelineKey::new(self, shader_key);

        let pipeline = match self.gltf.pipelines.get(&pipeline_key) {
            None => {

                let pipeline = self
                    .gpu
                    .create_render_pipeline(&pipeline_key.into_descriptor(&shader_module))
                    .await?;


                self.gltf
                    .pipelines
                    .insert(pipeline_key.clone(), pipeline.clone());

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
