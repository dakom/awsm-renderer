mod animations;
mod interpolate;
mod error;

pub use error::AwsmAnimationError;
pub use animations::{Animations, AnimationKey};

use glam::Quat;

#[derive(Debug, Clone)]
pub struct Animation {
    pub keyframes: Vec<AnimationKeyframe>,
}

#[derive(Debug, Clone)]
pub struct AnimationKeyframe {
    pub time: f32,
    pub kind: AnimationKind,
}

#[derive(Debug, Clone)]
pub enum AnimationKind {
    Transform(TransformAnimation),
    Morph(MorphAnimation),
}

#[derive(Debug, Clone)]
pub struct TransformAnimation {
    pub translation_x: Option<f32>,
    pub translation_y: Option<f32>,
    pub translation_z: Option<f32>,
    pub rotation: Option<Quat>,
    pub scale_x: Option<f32>,
    pub scale_y: Option<f32>,
    pub scale_z: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct MorphAnimation {
}