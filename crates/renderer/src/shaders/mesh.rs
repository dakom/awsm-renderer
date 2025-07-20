use std::collections::HashSet;

use crate::shaders::VertexLocation;


#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct MeshShaderCacheKeyGeometry {
    pub attributes: Vec<ShaderCacheKeyAttribute>,
    pub morphs: ShaderCacheKeyMorphs,
    pub has_instance_transforms: bool,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderCacheKeyAttribute {
    /// XYZ vertex positions.
    Positions,

    /// XYZ vertex normals.
    Normals,

    /// XYZW vertex tangents where the `w` component is a sign value indicating the
    /// handedness of the tangent basis.
    Tangents,

    /// RGB or RGBA vertex color.
    Colors { count: u32 },

    /// UV texture co-ordinates.
    TexCoords { count: u32 },

    /// Joint indices.
    Joints { count: u32 },

    /// Joint weights.
    Weights { count: u32 },
}

impl ShaderCacheKeyAttribute {
    pub fn count(&self) -> u32 {
        match self {
            ShaderCacheKeyAttribute::Positions => 1,
            ShaderCacheKeyAttribute::Normals => 1,
            ShaderCacheKeyAttribute::Tangents => 1,
            ShaderCacheKeyAttribute::Colors { count } => *count,
            ShaderCacheKeyAttribute::TexCoords { count } => *count,
            ShaderCacheKeyAttribute::Joints { count } => *count,
            ShaderCacheKeyAttribute::Weights { count } => *count,
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShaderCacheKeyMorphs {
    pub position: bool,
    pub normal: bool,
    pub tangent: bool,
}

impl ShaderCacheKeyMorphs {
    pub fn any(&self) -> bool {
        self.position || self.normal || self.tangent
    }
}

#[derive(Debug)]
pub struct MeshShaderTemplateGeometry {
    pub vertex_input_locations: Vec<VertexLocation>,

    // morphs
    pub morphs: ShaderCacheKeyMorphs,
    // skins
    pub skins: u32,

    // simpler ways of doing things
    pub has_normals: bool,
    pub has_instance_transforms: bool
}

impl From<MeshShaderCacheKeyGeometry> for MeshShaderTemplateGeometry {
    fn from(geometry: MeshShaderCacheKeyGeometry) -> Self {
        let mut has_normals = false;
        let mut skins = None;
        let mut vertex_input_locations = Vec::new();

        let mut sanity_check = HashSet::new();
        for attribute in &geometry.attributes {
            if !sanity_check.insert(std::mem::discriminant(attribute)) {
                panic!("Duplicate attribute found: {attribute:?}");
            }

            match attribute {
                ShaderCacheKeyAttribute::Normals => {
                    has_normals = true;
                }
                ShaderCacheKeyAttribute::Joints { count } => {
                    // joints and weights must always be equal
                    // each additional "count" allows up to 4 more vertex influences
                    skins = Some(*count);
                }
                _ => {}
            }

            for count in 0..attribute.count() {
                vertex_input_locations.push(VertexLocation {
                    location: vertex_input_locations.len() as u32,
                    interpolation: match attribute {
                        ShaderCacheKeyAttribute::Positions => None,
                        ShaderCacheKeyAttribute::Normals => None,
                        ShaderCacheKeyAttribute::Tangents => None,
                        ShaderCacheKeyAttribute::Colors { .. } => None,
                        ShaderCacheKeyAttribute::TexCoords { .. } => None,
                        ShaderCacheKeyAttribute::Joints { .. } => Some("flat"),
                        ShaderCacheKeyAttribute::Weights { .. } => None,
                    },
                    name: match attribute {
                        ShaderCacheKeyAttribute::Positions => "position".to_string(),
                        ShaderCacheKeyAttribute::Normals => "normal".to_string(),
                        ShaderCacheKeyAttribute::Tangents => "tangent".to_string(),
                        ShaderCacheKeyAttribute::Colors { .. } => format!("color_{count}"),
                        ShaderCacheKeyAttribute::TexCoords { .. } => format!("uv_{count}"),
                        ShaderCacheKeyAttribute::Joints { .. } => format!("skin_joint_{count}"),
                        ShaderCacheKeyAttribute::Weights { .. } => format!("skin_weight_{count}"),
                    },
                    data_type: match attribute {
                        ShaderCacheKeyAttribute::Positions => "vec3<f32>".to_string(),
                        ShaderCacheKeyAttribute::Normals => "vec3<f32>".to_string(),
                        ShaderCacheKeyAttribute::Tangents => "vec3<f32>".to_string(),
                        ShaderCacheKeyAttribute::Colors { .. } => "vec4<f32>".to_string(),
                        ShaderCacheKeyAttribute::TexCoords { .. } => "vec2<f32>".to_string(),
                        ShaderCacheKeyAttribute::Joints { .. } => "vec4<u32>".to_string(),
                        ShaderCacheKeyAttribute::Weights { .. } => "vec4<f32>".to_string(),
                    },
                });
            }
        }

        if geometry.has_instance_transforms {
            for i in 0..4 {
                vertex_input_locations.push(VertexLocation {
                    location: vertex_input_locations.len() as u32,
                    interpolation: None,
                    name: format!("instance_transform_row_{i}"),
                    data_type: "vec4<f32>".to_string(),
                });
            }
        }

        Self { 
            vertex_input_locations,
            morphs: geometry.morphs,
            skins: skins.unwrap_or(0),
            has_normals,
            has_instance_transforms: geometry.has_instance_transforms,
        }
    }
}