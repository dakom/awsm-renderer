use crate::AwsmRenderer;

use super::loader::GltfResource;

impl AwsmRenderer {
    pub async fn populate_gltf(&mut self, gltf_res: &GltfResource) -> anyhow::Result<()> {
        tracing::info!("Populating gltf resource...");
        // TODO - populate the gltf resource with the data from the gltf file
        // This will include loading textures, buffers, and other resources
        // and creating the necessary GPU resources for rendering

        // static INIT_SHADER_CODE: &str = include_str!("wip-shaders/init.wgsl");
        // let shader = self.gpu.compile_shader(&ShaderModuleDescriptor::new(INIT_SHADER_CODE, None).into());

        // let vertex = VertexState::new(&shader, None);
        // let fragment = FragmentState::new(
        //     &shader,
        //     None,
        //     vec![ColorTargetState::new(self.gpu.current_context_format())],
        // );

        // let pipeline_descriptor = RenderPipelineDescriptor::new(vertex, None).with_fragment(fragment);

        // tracing::info!("Creating pipeline...");

        // let pipeline = self.gpu.create_pipeline(&pipeline_descriptor.into()).await?;
        Ok(())
    }
}