use slotmap::{new_key_type, DenseSlotMap};

use crate::{mesh::MeshKey, transform::TransformKey, AwsmRenderer};

use super::{
    data::{AnimationData, AppliedAnimation},
    error::Result,
    player::AnimationPlayer,
    AwsmAnimationError,
};

new_key_type! {
    pub struct AnimationKey;
}

#[derive(Debug, Clone, Default)]
pub struct Animations {
    pub players: DenseSlotMap<AnimationKey, AnimationPlayer<AnimationData>>,
    pub transforms: DenseSlotMap<AnimationKey, TransformKey>,
    pub meshes: DenseSlotMap<AnimationKey, MeshKey>,
}

impl AwsmRenderer {
    pub fn update_animations(&mut self, global_time_delta: f64) -> Result<()> {
        for player in self.animations.players.values_mut() {
            player.update(global_time_delta)
        }

        for (animation_key, transform_key) in self.animations.transforms.iter() {
            let player = self
                .animations
                .players
                .get(animation_key)
                .ok_or(AwsmAnimationError::MissingKey(animation_key))?;
            let transform = self.transforms.get_local(*transform_key)?;
            match player.sample() {
                AnimationData::Transform(transform_animation) => {
                    let updated_transform = transform_animation.apply(transform.clone());
                    self.transforms
                        .set_local(*transform_key, updated_transform)?;
                }
                _ => {
                    return Err(AwsmAnimationError::WrongKind("weird, animation player has a transform key but the animation data is not a transform".to_string()));
                }
            }
        }

        for (animation_key, mesh_key) in self.animations.meshes.iter() {
            let player = self
                .animations
                .players
                .get(animation_key)
                .ok_or(AwsmAnimationError::MissingKey(animation_key))?;

            match player.sample() {
                AnimationData::Vertex(vertex_animation) => {
                    self.meshes.morphs.update_morph_weights_with(*mesh_key, vertex_animation.weights.len(), |target| {
                        target.copy_from_slice(&vertex_animation.weights);
                    });
                }
                _ => {
                    return Err(AwsmAnimationError::WrongKind("weird, animation player has a mesh key but the animation data is not for a mesh".to_string()));
                }
            }
        }

        Ok(())
    }
}
