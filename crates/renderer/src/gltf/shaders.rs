use crate::shaders::ShaderKey;

pub fn semantic_shader_location(semantic: gltf::Semantic) -> u32 {
    match semantic {
        gltf::Semantic::Positions => 0,
        gltf::Semantic::Normals => 1,
        gltf::Semantic::Tangents => 2,
        // TODO - not sure if these are right
        gltf::Semantic::Colors(index) => 3 + index,
        gltf::Semantic::TexCoords(index) => 4 + index,
        gltf::Semantic::Joints(index) => 8 + index,
        gltf::Semantic::Weights(index) => 12 + index,
    }
}

impl ShaderKey {
    pub fn gltf_primitive_new(primitive: &gltf::Primitive<'_>) -> Self {
        let mut key = Self::default();

        primitive.morph_targets().for_each(|morph_target| {
            if morph_target.positions().is_some() {
                key.morphs = true;
            }
            if morph_target.normals().is_some() {
                key.morphs = true;
            }
            if morph_target.tangents().is_some() {
                key.morphs = true;
            }
        });

        for (semantic, _accessor) in primitive.attributes() {
            match semantic {
                gltf::Semantic::Positions => {
                    key.position_attribute = true;
                }
                gltf::Semantic::Normals => {
                    key.normal_attribute = true;
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
                gltf::Semantic::Joints(_joint_index) => {
                    tracing::warn!("TODO - primitive joins");
                }
                gltf::Semantic::Weights(_weight_index) => {
                    tracing::warn!("TODO - primitive weights");
                }
            }
        }

        key
    }
}