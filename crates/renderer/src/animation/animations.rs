use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use slotmap::{new_key_type, DenseSlotMap};

use super::{Animation, error::{Result, AwsmAnimationError}};

new_key_type! {
    pub struct AnimationKey;
}

#[derive(Default)]
pub struct Animations {
    pub animations: DenseSlotMap<AnimationKey, Animation>,
}

impl Animations {
    // Just updates the properties, does not write to GPU, can be called multiple times a frame
    // for example with a virtual/fixed-timestep independent of the render loop
    pub fn update(&mut self, time: f64) -> Result<()> {
        for animation in self.animations.values_mut() {
        }
        Ok(())
    }

    // Writes the properties to GPU, should be called once per frame
    pub fn write_gpu(&mut self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        Ok(())
    }
}