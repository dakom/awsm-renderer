use askama::Template;
use std::collections::HashSet;

use crate::shaders::vertex::{
    entry::mesh::{
        ShaderCacheKeyVertexMesh, ShaderCacheKeyVertexMeshAttribute, ShaderCacheKeyVertexMeshMorphs,
    },
    ShaderTemplateVertexLocation, ShaderTemplateVertexToFragmentAssignment,
};

#[derive(Template, Debug)]
#[template(path = "vertex/mesh.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateVertexMesh {
    pub vertex_input_locations: Vec<ShaderTemplateVertexLocation>,
    pub vertex_to_fragment_assignments: Vec<ShaderTemplateVertexToFragmentAssignment>,

    // morphs
    pub morphs: ShaderCacheKeyVertexMeshMorphs,
    // skins
    pub skins: u32,

    // simpler ways of doing things
    pub has_normals: bool,
    pub has_instance_transforms: bool,
}

impl ShaderTemplateVertexMesh {
    pub fn new(cache_key: &ShaderCacheKeyVertexMesh) -> Self {
        let mut has_normals = false;
        let mut skins = None;
        let mut vertex_input_locations = Vec::new();

        let mut sanity_check = HashSet::new();
        for attribute in &cache_key.attributes {
            if !sanity_check.insert(std::mem::discriminant(attribute)) {
                panic!("Duplicate attribute found: {attribute:?}");
            }

            match attribute {
                ShaderCacheKeyVertexMeshAttribute::Normals => {
                    has_normals = true;
                }
                ShaderCacheKeyVertexMeshAttribute::Joints { count } => {
                    // joints and weights must always be equal
                    // each additional "count" allows up to 4 more vertex influences
                    skins = Some(*count);
                }
                _ => {}
            }

            for count in 0..attribute.count() {
                vertex_input_locations.push(ShaderTemplateVertexLocation {
                    location: vertex_input_locations.len() as u32,
                    interpolation: match attribute {
                        ShaderCacheKeyVertexMeshAttribute::Positions => None,
                        ShaderCacheKeyVertexMeshAttribute::Normals => None,
                        ShaderCacheKeyVertexMeshAttribute::Tangents => None,
                        ShaderCacheKeyVertexMeshAttribute::Colors { .. } => None,
                        ShaderCacheKeyVertexMeshAttribute::TexCoords { .. } => None,
                        ShaderCacheKeyVertexMeshAttribute::Joints { .. } => Some("flat"),
                        ShaderCacheKeyVertexMeshAttribute::Weights { .. } => None,
                    },
                    name: match attribute {
                        ShaderCacheKeyVertexMeshAttribute::Positions => "position".to_string(),
                        ShaderCacheKeyVertexMeshAttribute::Normals => "normal".to_string(),
                        ShaderCacheKeyVertexMeshAttribute::Tangents => "tangent".to_string(),
                        ShaderCacheKeyVertexMeshAttribute::Colors { .. } => {
                            format!("color_{count}")
                        }
                        ShaderCacheKeyVertexMeshAttribute::TexCoords { .. } => {
                            format!("uv_{count}")
                        }
                        ShaderCacheKeyVertexMeshAttribute::Joints { .. } => {
                            format!("skin_joint_{count}")
                        }
                        ShaderCacheKeyVertexMeshAttribute::Weights { .. } => {
                            format!("skin_weight_{count}")
                        }
                    },
                    data_type: match attribute {
                        ShaderCacheKeyVertexMeshAttribute::Positions => "vec3<f32>".to_string(),
                        ShaderCacheKeyVertexMeshAttribute::Normals => "vec3<f32>".to_string(),
                        ShaderCacheKeyVertexMeshAttribute::Tangents => "vec3<f32>".to_string(),
                        ShaderCacheKeyVertexMeshAttribute::Colors { .. } => "vec4<f32>".to_string(),
                        ShaderCacheKeyVertexMeshAttribute::TexCoords { .. } => {
                            "vec2<f32>".to_string()
                        }
                        ShaderCacheKeyVertexMeshAttribute::Joints { .. } => "vec4<u32>".to_string(),
                        ShaderCacheKeyVertexMeshAttribute::Weights { .. } => {
                            "vec4<f32>".to_string()
                        }
                    },
                });
            }
        }

        if cache_key.has_instance_transforms {
            for i in 0..4 {
                vertex_input_locations.push(ShaderTemplateVertexLocation {
                    location: vertex_input_locations.len() as u32,
                    interpolation: None,
                    name: format!("instance_transform_row_{i}"),
                    data_type: "vec4<f32>".to_string(),
                });
            }
        }

        Self {
            vertex_input_locations,
            vertex_to_fragment_assignments: Vec::new(),
            morphs: cache_key.morphs,
            skins: skins.unwrap_or(0),
            has_normals,
            has_instance_transforms: cache_key.has_instance_transforms,
        }
    }
}
