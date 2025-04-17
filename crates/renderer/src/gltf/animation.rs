use crate::animation::{AnimationClip, AnimationKey, AnimationPlayer, TransformAnimation};
use crate::mesh::MorphKey;
use crate::{transform::TransformKey, AwsmRenderer};

use super::populate::GltfPopulateContext;
use super::error::{Result, AwsmGltfError};

impl AwsmRenderer {
    pub(super) fn populate_gltf_animation_transform_translation<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_animation: &'b gltf::Animation<'b>,
        transform_key: TransformKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_transform(ctx, gltf_animation)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_transform(player, transform_key))
    }

    pub(super) fn populate_gltf_animation_transform_rotation<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_animation: &'b gltf::Animation<'b>,
        transform_key: TransformKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_transform(ctx, gltf_animation)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_transform(player, transform_key))
    }

    pub(super) fn populate_gltf_animation_transform_scale<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_animation: &'b gltf::Animation<'b>,
        transform_key: TransformKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_transform(ctx, gltf_animation)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_transform(player, transform_key))
    }

    pub(super) fn populate_gltf_animation_morph<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_animation: &'b gltf::Animation<'b>,
        morph_key: MorphKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_morph(ctx, gltf_animation)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_morph(player, morph_key))
    }
}

fn gltf_animation_clip_transform(
    ctx: &GltfPopulateContext,
    gltf_animation: &gltf::Animation,
) -> Result<AnimationClip> {
    Err(AwsmGltfError::Todo("create animation clip for transform".to_string()))
}

fn gltf_animation_clip_morph(
    ctx: &GltfPopulateContext,
    gltf_animation: &gltf::Animation,
) -> Result<AnimationClip> {
    Err(AwsmGltfError::Todo("create animation clip for morph".to_string()))
}