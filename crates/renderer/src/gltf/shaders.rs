use super::error::{AwsmGltfError, Result};
use crate::shaders::ShaderKey;

pub fn semantic_shader_location(semantic: gltf::Semantic) -> u32 {
    match semantic {
        gltf::Semantic::Positions => 0,
        gltf::Semantic::Normals => 1,
        gltf::Semantic::Tangents => 2,
        // TODO - not sure if these are right
        gltf::Semantic::Joints(_) => 3,
        gltf::Semantic::Weights(_) => 4,
        gltf::Semantic::TexCoords(index) => 5 + index,
        gltf::Semantic::Colors(index) => 10 + index,
    }
}

impl ShaderKey {
    pub fn gltf_primitive_new(primitive: &gltf::Primitive<'_>) -> Result<Self> {
        let mut key = Self::default();

        primitive.morph_targets().for_each(|morph_target| {
            if morph_target.positions().is_some() {
                key.has_morphs = true;
            }
            if morph_target.normals().is_some() {
                key.has_morphs = true;
            }
            if morph_target.tangents().is_some() {
                key.has_morphs = true;
            }
        });

        let mut joint_sets = 0;
        let mut weight_sets = 0;
        for (semantic, _accessor) in primitive.attributes() {
            match semantic {
                gltf::Semantic::Positions => {
                    key.has_position = true;
                }
                gltf::Semantic::Normals => {
                    key.has_normal = true;
                }
                gltf::Semantic::Tangents => {
                    tracing::warn!("TODO - primitive tangents");
                }
                gltf::Semantic::Colors(_color_index) => {
                    tracing::warn!("TODO - primitive colors");
                }
                gltf::Semantic::TexCoords(_uvs) => {
                    tracing::warn!("TODO - primitive uvs");
                }
                gltf::Semantic::Joints(joint_index) => {
                    joint_sets = joint_sets.max(joint_index + 1);
                }
                gltf::Semantic::Weights(weight_index) => {
                    weight_sets = weight_sets.max(weight_index + 1);
                }
            }
        }

        if joint_sets != weight_sets {
            return Err(AwsmGltfError::ShaderKeyDifferentJointsWeights {
                weight_sets,
                joint_sets,
            });
        }

        key.skin_joint_sets = joint_sets;

        Ok(key)
    }
}
