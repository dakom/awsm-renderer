use super::{data::AnimationData, sampler::AnimationSampler};

#[derive(Debug, Clone)]
pub struct AnimationClip<T = AnimationData> {
    pub name: Option<String>,
    pub duration: f64,
    pub sampler: AnimationSampler<T>,
}

impl<T> AnimationClip<T> {
    pub fn new(name: Option<String>, duration: f64, sampler: AnimationSampler<T>) -> Self {
        Self {
            name,
            duration,
            sampler,
        }
    }
}
