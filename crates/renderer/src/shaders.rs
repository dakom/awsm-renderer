use std::collections::{HashMap, HashSet};

use askama::Template;
use awsm_renderer_core::{
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
    shaders::{ShaderModuleDescriptor, ShaderModuleExt},
};
use thiserror::Error;

pub struct Shaders {
    cache: HashMap<ShaderCacheKey, web_sys::GpuShaderModule>,
}

impl Default for Shaders {
    fn default() -> Self {
        Self::new()
    }
}

impl Shaders {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub async fn get_or_create(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        key: &ShaderCacheKey,
    ) -> Result<web_sys::GpuShaderModule> {
        match self.cache.get(key) {
            None => {
                let shader_module = gpu.compile_shader(&key.into_descriptor()?);
                shader_module
                    .validate_shader()
                    .await
                    .map_err(AwsmShaderError::Compilation)?;

                self.cache.insert(key.clone(), shader_module.clone());

                Ok(shader_module)
            }
            Some(shader_module) => Ok(shader_module.clone()),
        }
    }
}

#[repr(u16)]
pub enum ShaderConstantIds {
    MorphTargetLen = 1,
}
// merely a key to hash ad-hoc shader generation
// is not stored on the mesh itself
//
// uniform and other runtime data for mesh
// is controlled via various components as-needed
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKey {
    pub attributes: Vec<ShaderCacheKeyAttribute>,
    pub morphs: ShaderCacheKeyMorphs,
    pub instancing: ShaderCacheKeyInstancing,
    pub material: ShaderCacheKeyMaterial,
}

impl ShaderCacheKey {
    pub fn new(attributes: Vec<ShaderCacheKeyAttribute>, material: ShaderCacheKeyMaterial) -> Self {
        Self {
            attributes,
            morphs: Default::default(),
            instancing: Default::default(),
            material,
        }
    }

    pub fn with_morphs(mut self, morphs: ShaderCacheKeyMorphs) -> Self {
        self.morphs = morphs;
        self
    }

    pub fn with_instancing(mut self, instancing: ShaderCacheKeyInstancing) -> Self {
        self.instancing = instancing;
        self
    }

    pub fn with_material(mut self, material: ShaderCacheKeyMaterial) -> Self {
        self.material = material;
        self
    }
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

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShaderCacheKeyInstancing {
    pub transform: bool,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderCacheKeyMaterial {
    Pbr(PbrShaderCacheKeyMaterial),
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PbrShaderCacheKeyMaterial {
    pub base_color_uv_index: Option<u32>,
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

impl ShaderCacheKey {
    pub fn into_descriptor(&self) -> Result<web_sys::GpuShaderModuleDescriptor> {
        Ok(ShaderModuleDescriptor::new(&self.into_source()?, None).into())
    }

    pub fn into_source(&self) -> Result<String> {
        let mut material = ShaderTemplateMaterial::default();
        let mut skins = None;

        let mut vertex_input_locations = Vec::new();
        let mut sanity_check = HashSet::new();
        for attribute in &self.attributes {
            if !sanity_check.insert(std::mem::discriminant(attribute)) {
                panic!("Duplicate attribute found: {:?}", attribute);
            }

            for count in 0..attribute.count() {
                match attribute {
                    ShaderCacheKeyAttribute::Normals => {
                        material.has_normal = true;
                    }
                    ShaderCacheKeyAttribute::Joints { count } => {
                        skins = Some(*count);
                    }
                    _ => {}
                }
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

        if self.instancing.transform {
            for i in 0..4 {
                vertex_input_locations.push(VertexLocation {
                    location: vertex_input_locations.len() as u32,
                    interpolation: None,
                    name: format!("instance_transform_row_{i}"),
                    data_type: "vec4<f32>".to_string(),
                });
            }
        }

        let mut vertex_to_fragment_assignments = Vec::new();
        let mut vertex_output_locations = Vec::new();
        let mut fragment_buffer_bindings = Vec::new();

        match self.material {
            ShaderCacheKeyMaterial::Pbr(material_key) => {
                if let Some(uv_index) = material_key.base_color_uv_index {
                    fragment_buffer_bindings.push(DynamicBufferBinding {
                        group: 2,
                        index: fragment_buffer_bindings.len() as u32,
                        name: "base_color_tex".to_string(),
                        data_type: "texture_2d<f32>".to_string(),
                    });

                    fragment_buffer_bindings.push(DynamicBufferBinding {
                        group: 2,
                        index: fragment_buffer_bindings.len() as u32,
                        name: "base_color_sampler".to_string(),
                        data_type: "sampler".to_string(),
                    });

                    vertex_output_locations.push(VertexLocation {
                        location: vertex_output_locations.len() as u32,
                        interpolation: None,
                        name: "base_color_uv".to_string(),
                        data_type: "vec2<f32>".to_string(),
                    });

                    vertex_to_fragment_assignments.push(VertexToFragmentAssignment {
                        vertex_name: format!("uv_{uv_index}"),
                        fragment_name: "base_color_uv".to_string(),
                    });

                    material.has_base_color = true;
                }

                if material.has_normal {
                    vertex_output_locations.push(VertexLocation {
                        location: vertex_output_locations.len() as u32,
                        interpolation: None,
                        name: "normal".to_string(),
                        data_type: "vec3<f32>".to_string(),
                    });
                    vertex_to_fragment_assignments.push(VertexToFragmentAssignment {
                        vertex_name: "normal".to_string(),
                        fragment_name: "normal".to_string(),
                    });
                }
            }
        }

        let tmpl = ShaderTemplate {
            vertex_input_locations,
            vertex_output_locations,
            vertex_to_fragment_assignments,
            morphs: self.morphs,
            skins: skins.unwrap_or_default(),
            has_instance_transform: self.instancing.transform,
            fragment_shader_kind: FragmentShaderKind::Pbr,
            //fragment_shader_kind: FragmentShaderKind::DebugNormals,
            fragment_buffer_bindings,
            material,
        };

        let source = tmpl.render().unwrap();

        Ok(source)
    }
}

#[derive(Template, Debug)]
#[template(path = "main.wgsl", whitespace = "minimize")]
struct ShaderTemplate {
    // location, interpolation, name, data-type
    pub vertex_input_locations: Vec<VertexLocation>,
    pub vertex_output_locations: Vec<VertexLocation>,
    pub fragment_buffer_bindings: Vec<DynamicBufferBinding>,
    pub vertex_to_fragment_assignments: Vec<VertexToFragmentAssignment>,
    // morphs
    pub morphs: ShaderCacheKeyMorphs,
    // skins
    pub skins: u32,

    // simpler ways of doing things
    pub has_instance_transform: bool,
    pub fragment_shader_kind: FragmentShaderKind,
    pub material: ShaderTemplateMaterial,
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

#[derive(Debug, Default)]
pub struct ShaderTemplateMaterial {
    // the idea here is that with these gates, we can write normal shader code
    // since the variables are assigned (and from then on, we don't care about the location)
    pub has_base_color: bool,
    pub has_normal: bool,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentShaderKind {
    DebugNormals,
    Pbr,
}

#[derive(Debug)]
struct VertexLocation {
    location: u32,
    interpolation: Option<&'static str>,
    name: String,
    data_type: String,
}

#[derive(Debug)]
struct DynamicBufferBinding {
    group: u32,
    index: u32,
    name: String,
    data_type: String,
}

#[derive(Debug)]
struct VertexToFragmentAssignment {
    vertex_name: String,
    fragment_name: String,
}

type Result<T> = std::result::Result<T, AwsmShaderError>;
#[derive(Error, Debug)]
pub enum AwsmShaderError {
    #[error("[shader] source error: {0}")]
    DuplicateAttribute(String),

    #[error("[shader] Compilation error: {0:?}")]
    Compilation(AwsmCoreError),
}
