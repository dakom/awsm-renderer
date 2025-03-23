use awsm_renderer_core::renderer::AwsmRendererWebGpu;

pub struct AwsmRendererScene {
    pub gpu: AwsmRendererWebGpu,
}

impl AwsmRendererScene {
    pub fn new(gpu: AwsmRendererWebGpu) -> Self {
        Self { gpu}
    }
}