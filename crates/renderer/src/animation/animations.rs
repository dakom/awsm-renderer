use slotmap::{new_key_type, DenseSlotMap, SecondaryMap};

use crate::{mesh::MorphKey, transform::TransformKey, AwsmRenderer};

use super::{data::AnimationData, error::Result, player::AnimationPlayer, AwsmAnimationError};

new_key_type! {
    pub struct AnimationKey;
}

#[derive(Debug, Clone, Default)]
pub struct Animations {
    players: DenseSlotMap<AnimationKey, AnimationPlayer>,
    transforms: SecondaryMap<AnimationKey, TransformKey>,
    morphs: SecondaryMap<AnimationKey, MorphKey>,
}

impl Animations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remove(&mut self, key: AnimationKey) {
        self.players.remove(key);
        self.transforms.remove(key);
        self.morphs.remove(key);
    }

    pub fn insert_transform(
        &mut self,
        player: AnimationPlayer,
        transform_key: TransformKey,
    ) -> AnimationKey {
        let key = self.players.insert(player);
        self.transforms.insert(key, transform_key);
        key
    }

    pub fn insert_morph(&mut self, player: AnimationPlayer, morph_key: MorphKey) -> AnimationKey {
        let key = self.players.insert(player);
        self.morphs.insert(key, morph_key);
        key
    }
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

        for (animation_key, morph_key) in self.animations.morphs.iter() {
            let player = self
                .animations
                .players
                .get(animation_key)
                .ok_or(AwsmAnimationError::MissingKey(animation_key))?;

            match player.sample() {
                AnimationData::Vertex(vertex_animation) => {
                    self.meshes
                        .morphs
                        .update_morph_weights_with(*morph_key, |target| {
                            target.copy_from_slice(&vertex_animation.weights);
                        })?;
                }
                _ => {
                    return Err(AwsmAnimationError::WrongKind("weird, animation player has a mesh key but the animation data is not for a mesh".to_string()));
                }
            }
        }

        Ok(())
    }
}
