mod animations;
mod clip;
mod error;
mod interpolate;
mod player;
mod data;

pub use animations::{Animations, AnimationKey};
pub use player::{AnimationPlayer, AnimationState};
pub use clip::AnimationClip;
pub use error::AwsmAnimationError;