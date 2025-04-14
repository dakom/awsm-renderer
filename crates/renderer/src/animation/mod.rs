mod animations;
mod clip;
mod data;
mod error;
mod interpolate;
mod player;

pub use animations::{AnimationKey, Animations};
pub use clip::AnimationClip;
pub use error::AwsmAnimationError;
pub use player::{AnimationPlayer, AnimationState};
