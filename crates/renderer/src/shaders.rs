use std::collections::HashSet;

use askama::Template;
use awsm_renderer_core::shaders::ShaderModuleDescriptor;
use thiserror::Error;

#[repr(u16)]
pub enum ShaderConstantIds {
    MorphTargetLen = 1,
}
// merely a key to hash ad-hoc shader generation
// is not stored on the mesh itself
//
// uniform and other runtime data for mesh
// is controlled via various components as-needed
#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub struct ShaderKey {
    attributes: Vec<ShaderKeyAttribute>,
    morphs: ShaderKeyMorphs,
}

impl ShaderKey {
    pub fn new(attributes: Vec<ShaderKeyAttribute>, morphs: Option<ShaderKeyMorphs>) -> Self {
        Self {
            attributes,
            morphs: morphs.unwrap_or_default(),
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderKeyAttribute {
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

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShaderKeyMorphs {
    pub position: bool,
    pub normal: bool,
    pub tangent: bool,
}

impl ShaderKeyMorphs {
    pub fn any(&self) -> bool {
        self.position || self.normal || self.tangent
    }
}

impl ShaderKeyAttribute {
    pub fn count(&self) -> u32 {
        match self {
            ShaderKeyAttribute::Positions => 1,
            ShaderKeyAttribute::Normals => 1,
            ShaderKeyAttribute::Tangents => 1,
            ShaderKeyAttribute::Colors { count } => *count,
            ShaderKeyAttribute::TexCoords { count } => *count,
            ShaderKeyAttribute::Joints { count } => *count,
            ShaderKeyAttribute::Weights { count } => *count,
        }
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct SkinTarget {
    pub weight_loc: u32,
    pub joint_loc: u32,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct VertexColor {
    pub loc: u32,
    pub size: VertexColorSize,
}
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum VertexColorSize {
    Vec3,
    Vec4,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderKeyAlphaMode {
    Opaque,
    Blend,
    Mask,
}

impl Default for ShaderKeyAlphaMode {
    fn default() -> Self {
        Self::Opaque
    }
}

impl ShaderKey {
    pub fn into_descriptor(&self) -> Result<web_sys::GpuShaderModuleDescriptor> {
        Ok(ShaderModuleDescriptor::new(&self.into_source()?, None).into())
    }

    pub fn into_source(&self) -> Result<String> {
        let mut shader_location: u32 = 0;

        let mut vertex_input_locations = Vec::new();
        let mut sanity_check = HashSet::new();
        for attribute in &self.attributes {
            if !sanity_check.insert(std::mem::discriminant(attribute)) {
                panic!("Duplicate attribute found: {:?}", attribute);
            }

            for count in 0..attribute.count() {
                vertex_input_locations.push(VertexInputLocation {
                    location: shader_location,
                    interpolation: match attribute {
                        ShaderKeyAttribute::Positions => None,
                        ShaderKeyAttribute::Normals => None,
                        ShaderKeyAttribute::Tangents => None,
                        ShaderKeyAttribute::Colors { .. } => None,
                        ShaderKeyAttribute::TexCoords { .. } => None,
                        ShaderKeyAttribute::Joints { .. } => Some("flat"),
                        ShaderKeyAttribute::Weights { .. } => None,
                    },
                    name: match attribute {
                        ShaderKeyAttribute::Positions => "position".to_string(),
                        ShaderKeyAttribute::Normals => "normal".to_string(),
                        ShaderKeyAttribute::Tangents => "tangent".to_string(),
                        ShaderKeyAttribute::Colors { .. } => format!("color_{count}"),
                        ShaderKeyAttribute::TexCoords { .. } => format!("texcoord_{count}"),
                        ShaderKeyAttribute::Joints { .. } => format!("skin_joint_{count}"),
                        ShaderKeyAttribute::Weights { .. } => format!("skin_weight_{count}"),
                    },
                    data_type: match attribute {
                        ShaderKeyAttribute::Positions => "vec3<f32>".to_string(),
                        ShaderKeyAttribute::Normals => "vec3<f32>".to_string(),
                        ShaderKeyAttribute::Tangents => "vec3<f32>".to_string(),
                        ShaderKeyAttribute::Colors { .. } => "vec4<f32>".to_string(),
                        ShaderKeyAttribute::TexCoords { .. } => "vec2<f32>".to_string(),
                        ShaderKeyAttribute::Joints { .. } => "vec4<u32>".to_string(),
                        ShaderKeyAttribute::Weights { .. } => "vec4<f32>".to_string(),
                    },
                });
                shader_location += 1;
            }
        }

        let tmpl = ShaderTemplate {
            vertex_input_locations,
            morphs: self.morphs,
            skins: self
                .attributes
                .iter()
                .find_map(|a| {
                    if let ShaderKeyAttribute::Joints { count } = a {
                        Some(*count)
                    } else {
                        None
                    }
                })
                .unwrap_or_default(),
        };

        let source = tmpl.render().unwrap();

        Ok(source)
    }
}

#[derive(Template, Debug, Default)]
#[template(path = "main.wgsl", whitespace = "minimize")]
struct ShaderTemplate {
    // location, interpolation, name, data-type
    pub vertex_input_locations: Vec<VertexInputLocation>,
    // morphs
    pub morphs: ShaderKeyMorphs,
    // skins
    pub skins: u32,
    // pub skin_targets: Vec<SkinTarget>,
    // pub n_skin_joints: u8,
    // pub tex_coords: Option<Vec<u32>>,
    // pub vertex_colors: Option<Vec<VertexColor>>,
    // pub normal_texture_uv_index: Option<u32>,
    // pub metallic_roughness_texture_uv_index: Option<u32>,
    // pub base_color_texture_uv_index: Option<u32>,
    // pub emissive_texture_uv_index: Option<u32>,
    // pub alpha_mode: ShaderKeyAlphaMode,
}

#[derive(Debug)]
struct VertexInputLocation {
    location: u32,
    interpolation: Option<&'static str>,
    name: String,
    data_type: String,
}

type Result<T> = std::result::Result<T, AwsmShaderError>;
#[derive(Error, Debug)]
pub enum AwsmShaderError {
    #[error("Shader source error: {0}")]
    DuplicateAttribute(String),
}
