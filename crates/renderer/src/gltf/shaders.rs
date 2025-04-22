use super::error::{AwsmGltfError, Result};
use crate::shaders::ShaderKey;

pub fn semantic_shader_location(semantic: gltf::Semantic) -> Result<u32> {
    match semantic {
        gltf::Semantic::Positions => Ok(0),
        gltf::Semantic::Normals => Ok(1),
        gltf::Semantic::Tangents => Ok(2),
        gltf::Semantic::Joints(0) => Ok(3),
        gltf::Semantic::Weights(0) => Ok(4),
        gltf::Semantic::Joints(1) => Ok(5),
        gltf::Semantic::Weights(1) => Ok(6),
        gltf::Semantic::Joints(2) => Ok(7),
        gltf::Semantic::Weights(2) => Ok(8),
        gltf::Semantic::TexCoords(_) => Ok(9),
        gltf::Semantic::Colors(_) => Ok(10),
        _ => Err(AwsmGltfError::ShaderLocationNoSemantic(semantic)),
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
