use std::collections::HashSet;

use askama::Template;

use crate::{render_passes::{geometry::shader::cache_key::{ShaderCacheKeyGeometry, ShaderCacheKeyGeometryAttribute, ShaderCacheKeyGeometryMorphs}, material::template::{ShaderTemplateVertexLocation, ShaderTemplateVertexToFragmentAssignment}}, shaders::{AwsmShaderError, Result}};


#[derive(Debug)]
pub struct ShaderTemplateGeometry {
    pub vertex: ShaderTemplateGeometryVertex,
    pub fragment: ShaderTemplateGeometryFragment,
}

#[derive(Template, Debug)]
#[template(path = "geometry_wgsl/vertex.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateGeometryVertex {
    pub vertex_input_locations: Vec<ShaderTemplateVertexLocation>,

    // morphs
    pub morphs: ShaderCacheKeyGeometryMorphs,
    // skins
    pub skins: u32,

    // simpler ways of doing things
    pub has_normals: bool,
    pub has_instance_transforms: bool,
}

impl ShaderTemplateGeometryVertex {
    pub fn new(cache_key: &ShaderCacheKeyGeometry) -> Self {
                let mut has_normals = false;
        let mut skins = None;
        let mut vertex_input_locations = Vec::new();

        let mut sanity_check = HashSet::new();
        for attribute in &cache_key.attributes {
            if !sanity_check.insert(std::mem::discriminant(attribute)) {
                panic!("Duplicate attribute found: {attribute:?}");
            }

            match attribute {
                ShaderCacheKeyGeometryAttribute::Normals => {
                    has_normals = true;
                }
                ShaderCacheKeyGeometryAttribute::Joints { count } => {
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
                        ShaderCacheKeyGeometryAttribute::Positions => None,
                        ShaderCacheKeyGeometryAttribute::Normals => None,
                        ShaderCacheKeyGeometryAttribute::Tangents => None,
                        ShaderCacheKeyGeometryAttribute::Colors { .. } => None,
                        ShaderCacheKeyGeometryAttribute::TexCoords { .. } => None,
                        ShaderCacheKeyGeometryAttribute::Joints { .. } => Some("flat"),
                        ShaderCacheKeyGeometryAttribute::Weights { .. } => None,
                    },
                    name: match attribute {
                        ShaderCacheKeyGeometryAttribute::Positions => "position".to_string(),
                        ShaderCacheKeyGeometryAttribute::Normals => "normal".to_string(),
                        ShaderCacheKeyGeometryAttribute::Tangents => "tangent".to_string(),
                        ShaderCacheKeyGeometryAttribute::Colors { .. } => {
                            format!("color_{count}")
                        }
                        ShaderCacheKeyGeometryAttribute::TexCoords { .. } => {
                            format!("uv_{count}")
                        }
                        ShaderCacheKeyGeometryAttribute::Joints { .. } => {
                            format!("skin_joint_{count}")
                        }
                        ShaderCacheKeyGeometryAttribute::Weights { .. } => {
                            format!("skin_weight_{count}")
                        }
                    },
                    data_type: match attribute {
                        ShaderCacheKeyGeometryAttribute::Positions => "vec3<f32>".to_string(),
                        ShaderCacheKeyGeometryAttribute::Normals => "vec3<f32>".to_string(),
                        ShaderCacheKeyGeometryAttribute::Tangents => "vec3<f32>".to_string(),
                        ShaderCacheKeyGeometryAttribute::Colors { .. } => "vec4<f32>".to_string(),
                        ShaderCacheKeyGeometryAttribute::TexCoords { .. } => {
                            "vec2<f32>".to_string()
                        }
                        ShaderCacheKeyGeometryAttribute::Joints { .. } => "vec4<u32>".to_string(),
                        ShaderCacheKeyGeometryAttribute::Weights { .. } => {
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
            morphs: cache_key.morphs,
            skins: skins.unwrap_or(0),
            has_normals,
            has_instance_transforms: cache_key.has_instance_transforms,
        }

    }
}

#[derive(Template, Debug)]
#[template(path = "geometry_wgsl/fragment.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateGeometryFragment {
    pub has_normals: bool
}

impl ShaderTemplateGeometryFragment {
    pub fn new(cache_key: &ShaderCacheKeyGeometry) -> Self {
        Self {
            has_normals: cache_key.attributes.contains(&ShaderCacheKeyGeometryAttribute::Normals),
        }
    }
}


impl TryFrom<&ShaderCacheKeyGeometry> for ShaderTemplateGeometry {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyGeometry) -> Result<Self> {
        Ok(Self {
            vertex: ShaderTemplateGeometryVertex::new(value),
            fragment: ShaderTemplateGeometryFragment::new(value),
        })
    }
}


impl ShaderTemplateGeometry {
    pub fn into_source(self) -> Result<String> {
        let vertex_source = self.vertex.render()?;
        let fragment_source = self.fragment.render()?;
        Ok(format!("{}\n{}", vertex_source, fragment_source))
    }
}