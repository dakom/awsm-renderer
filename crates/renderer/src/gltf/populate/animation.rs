use glam::{Quat, Vec3};

use crate::{
    animation::{
        AnimationClip, AnimationData, AnimationKey, AnimationPlayer, AnimationSampler,
        TransformAnimation, VertexAnimation,
    },
    buffer::helpers::u8_to_f32_vec,
    gltf::{
        buffers::accessor::accessor_to_bytes,
        error::{AwsmGltfError, Result},
    },
    mesh::MorphKey,
    transforms::TransformKey,
    AwsmRenderer,
};

use super::GltfPopulateContext;

impl AwsmRenderer {
    pub(super) fn populate_gltf_node_animation<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_node: &'b gltf::Node<'b>,
    ) -> Result<()> {
        let transform_key = ctx
            .node_to_transform
            .lock()
            .unwrap()
            .get(&gltf_node.index())
            .cloned()
            .unwrap();

        for gltf_animation in ctx.data.doc.animations() {
            for channel in gltf_animation.channels() {
                if channel.target().node().index() == gltf_node.index() {
                    match channel.target().property() {
                        gltf::animation::Property::Translation => {
                            self.populate_gltf_animation_transform_translation(
                                ctx,
                                gltf_animation
                                    .samplers()
                                    .nth(channel.sampler().index())
                                    .ok_or(AwsmGltfError::MissingAnimationSampler {
                                        animation_index: gltf_animation.index(),
                                        channel_index: channel.index(),
                                        sampler_index: channel.sampler().index(),
                                    })?,
                                transform_key,
                            )?;
                        }
                        gltf::animation::Property::Rotation => {
                            self.populate_gltf_animation_transform_rotation(
                                ctx,
                                gltf_animation
                                    .samplers()
                                    .nth(channel.sampler().index())
                                    .ok_or(AwsmGltfError::MissingAnimationSampler {
                                        animation_index: gltf_animation.index(),
                                        channel_index: channel.index(),
                                        sampler_index: channel.sampler().index(),
                                    })?,
                                transform_key,
                            )?;
                        }
                        gltf::animation::Property::Scale => {
                            self.populate_gltf_animation_transform_scale(
                                ctx,
                                gltf_animation
                                    .samplers()
                                    .nth(channel.sampler().index())
                                    .ok_or(AwsmGltfError::MissingAnimationSampler {
                                        animation_index: gltf_animation.index(),
                                        channel_index: channel.index(),
                                        sampler_index: channel.sampler().index(),
                                    })?,
                                transform_key,
                            )?;
                        }
                        gltf::animation::Property::MorphTargetWeights => {
                            // morph targets will be dealt with later when we populate the mesh
                            // by calling populate_gltf_animation_morph
                        }
                    }
                }
            }
        }

        for child in gltf_node.children() {
            self.populate_gltf_node_animation(ctx, &child)?;
        }

        Ok(())
    }

    pub(super) fn populate_gltf_animation_morph<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_sampler: gltf::animation::Sampler<'b>,
        morph_key: MorphKey,
    ) -> Result<AnimationKey> {
        let morph_info = &self.meshes.morphs.get_info(morph_key)?;

        let times = sampler_timestamps(ctx, &gltf_sampler)?;
        let duration = (times.last().copied().unwrap_or(0.0) - times[0]) as f64;
        let values = accessor_to_bytes(&gltf_sampler.output(), &ctx.data.buffers.raw)?;
        let values = u8_to_f32_vec(&values);

        let values = values
            .chunks(morph_info.targets_len)
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

    fn populate_gltf_animation_transform_translation<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_sampler: gltf::animation::Sampler<'b>,
        transform_key: TransformKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_transform(ctx, &gltf_sampler, TransformTarget::Translation)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_transform(player, transform_key))
    }

    fn populate_gltf_animation_transform_rotation<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_sampler: gltf::animation::Sampler<'b>,
        transform_key: TransformKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_transform(ctx, &gltf_sampler, TransformTarget::Rotation)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_transform(player, transform_key))
    }

    fn populate_gltf_animation_transform_scale<'a, 'b: 'a, 'c: 'a>(
        &'a mut self,
        ctx: &'c GltfPopulateContext,
        gltf_sampler: gltf::animation::Sampler<'b>,
        transform_key: TransformKey,
    ) -> Result<AnimationKey> {
        let clip = gltf_animation_clip_transform(ctx, &gltf_sampler, TransformTarget::Scale)?;
        let player = AnimationPlayer::new(clip);

        Ok(self.animations.insert_transform(player, transform_key))
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

enum TransformTarget {
    Translation,
    Rotation,
    Scale,
}

impl TransformTarget {
    fn as_str(&self) -> &'static str {
        match self {
            TransformTarget::Translation => "translation",
            TransformTarget::Rotation => "rotation",
            TransformTarget::Scale => "scale",
        }
    }

    fn chunk_size(&self) -> usize {
        match self {
            TransformTarget::Translation => 3,
            TransformTarget::Rotation => 4,
            TransformTarget::Scale => 3,
        }
    }
}

fn gltf_animation_clip_transform(
    ctx: &GltfPopulateContext,
    gltf_sampler: &gltf::animation::Sampler,
    target: TransformTarget,
) -> Result<AnimationClip> {
    let times = sampler_timestamps(ctx, gltf_sampler)?;
    let duration = (times.last().copied().unwrap_or(0.0) - times[0]) as f64;
    let values = accessor_to_bytes(&gltf_sampler.output(), &ctx.data.buffers.raw)?;
    let values = u8_to_f32_vec(&values);

    let values = values
        .chunks(target.chunk_size())
        .map(|chunk| {
            AnimationData::Transform(match target {
                TransformTarget::Translation => {
                    TransformAnimation::new_translation(Vec3::from_slice(chunk))
                }
                TransformTarget::Rotation => {
                    TransformAnimation::new_rotation(Quat::from_slice(chunk))
                }
                TransformTarget::Scale => TransformAnimation::new_scale(Vec3::from_slice(chunk)),
            })
        })
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

    Ok(AnimationClip::new(
        Some(format!("transform {}", target.as_str())),
        duration,
        sampler,
    ))
}
