use crate::animation::{
    AnimationClip, AnimationData, AnimationKey, AnimationPlayer, AnimationSampler,
    TransformAnimation, VertexAnimation,
};
use crate::buffers::helpers::u8_to_f32_vec;
use crate::mesh::MorphKey;
use crate::{transform::TransformKey, AwsmRenderer};

use super::buffers::accessor_to_bytes;
use super::error::{AwsmGltfError, Result};
use super::populate::GltfPopulateContext;

impl AwsmRenderer {
    pub(super) fn populate_gltf_animation_transform_translation<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_sampler: gltf::animation::Sampler<'b>,
        transform_key: TransformKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_transform(ctx, &gltf_sampler)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_transform(player, transform_key))
    }

    pub(super) fn populate_gltf_animation_transform_rotation<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_sampler: gltf::animation::Sampler<'b>,
        transform_key: TransformKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_transform(ctx, &gltf_sampler)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_transform(player, transform_key))
    }

    pub(super) fn populate_gltf_animation_transform_scale<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_sampler: gltf::animation::Sampler<'b>,
        transform_key: TransformKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_transform(ctx, &gltf_sampler)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_transform(player, transform_key))
    }

    pub(super) fn populate_gltf_animation_morph<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_sampler: gltf::animation::Sampler<'b>,
        morph_key: MorphKey,
    ) -> Result<AnimationKey> {
        let morph_info = &self.meshes.morphs.get_info(morph_key)?.morph_buffer_info;

        let times = sampler_timestamps(ctx, &gltf_sampler)?;
        let duration = (times.last().copied().unwrap_or(0.0) - times[0]) as f64;
        let values = accessor_to_bytes(&gltf_sampler.output(), &ctx.data.buffers.raw)?;
        let values = u8_to_f32_vec(&values);

        let values = values
            .chunks(morph_info.targets_len)
            .into_iter()
            .map(|chunk| AnimationData::Vertex(VertexAnimation::new(chunk.to_vec())))
            .collect();

        let sampler = match gltf_sampler.interpolation() {
            gltf::animation::Interpolation::Linear => AnimationSampler::Linear { times, values },
            gltf::animation::Interpolation::Step => AnimationSampler::Step { times, values },
            gltf::animation::Interpolation::CubicSpline => {
                let mut in_tangents = Vec::with_capacity(values.len() / 3);
                let mut spline_vertices = Vec::with_capacity(values.len() / 3);
                let mut out_tangents = Vec::with_capacity(values.len() / 3);

                for x in values.chunks_exact(3) {
                    in_tangents.push(x[0].clone());
                    spline_vertices.push(x[1].clone());
                    out_tangents.push(x[2].clone());
                }

                AnimationSampler::CubicSpline {
                    times,
                    in_tangents,
                    values: spline_vertices,
                    out_tangents,
                }
            }
        };

        let clip = AnimationClip::new(Some("morph".to_string()), duration, sampler);

        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_morph(player, morph_key))
    }
}

fn sampler_timestamps(
    ctx: &GltfPopulateContext,
    gltf_sampler: &gltf::animation::Sampler,
) -> Result<Vec<f64>> {
    let bytes = accessor_to_bytes(&gltf_sampler.input(), &ctx.data.buffers.raw)?;
    Ok(u8_to_f32_vec(&bytes)
        .into_iter()
        .map(|v| v as f64)
        .collect())
}

fn gltf_animation_clip_transform(
    ctx: &GltfPopulateContext,
    gltf_sampler: &gltf::animation::Sampler,
) -> Result<AnimationClip> {
    Err(AwsmGltfError::Todo(
        "create animation clip for transform".to_string(),
    ))
}

fn gltf_animation_clip_morph(
    ctx: &GltfPopulateContext,
    gltf_animation: &gltf::Animation,
) -> Result<AnimationClip> {
    Err(AwsmGltfError::Todo(
        "create animation clip for morph".to_string(),
    ))
}
