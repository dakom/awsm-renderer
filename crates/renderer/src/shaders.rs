use std::collections::{HashMap, HashSet};

use askama::Template;
use awsm_renderer_core::{
    error::AwsmCoreError,
    shaders::{ShaderModuleDescriptor, ShaderModuleExt},
};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::AwsmRenderer;

pub struct Shaders {
    lookup: SlotMap<ShaderKey, web_sys::GpuShaderModule>,
    cache: HashMap<ShaderCacheKey, ShaderKey>,
    reverse_cache: HashMap<ShaderKey, ShaderCacheKey>,
}

impl Default for Shaders {
    fn default() -> Self {
        Self::new()
    }
}

impl Shaders {
    pub fn new() -> Self {
        Self {
            lookup: SlotMap::with_key(),
            cache: HashMap::new(),
            reverse_cache: HashMap::new(),
        }
    }

    pub fn get_shader(&self, shader_key: ShaderKey) -> Option<&web_sys::GpuShaderModule> {
        self.lookup.get(shader_key)
    }

    pub fn get_shader_key_from_cache(&self, cache_key: &ShaderCacheKey) -> Option<ShaderKey> {
        self.cache.get(cache_key).cloned()
    }

    pub fn get_shader_cache_from_key(&self, key: &ShaderKey) -> Option<ShaderCacheKey> {
        self.reverse_cache.get(key).cloned()
    }
}

impl AwsmRenderer {
    pub async fn add_shader(&mut self, cache_key: ShaderCacheKey) -> Result<ShaderKey> {
        if let Some(shader_key) = self.shaders.get_shader_key_from_cache(&cache_key) {
            return Ok(shader_key);
        }

        let shader_module = self.gpu.compile_shader(&cache_key.into_descriptor()?);
        shader_module
            .validate_shader()
            .await
            .map_err(AwsmShaderError::Compilation)?;

        let shader_key = self.shaders.lookup.insert(shader_module.clone());

        self.shaders.cache.insert(cache_key.clone(), shader_key);
        self.shaders.reverse_cache.insert(shader_key, cache_key);

        Ok(shader_key)
    }
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
    FullScreenQuad,
    DebugNormals,
}

impl ShaderCacheKeyMaterial {
    pub fn has_alpha_mask(&self) -> bool {
        match self {
            ShaderCacheKeyMaterial::Pbr(material_key) => material_key.has_alpha_mask,
            ShaderCacheKeyMaterial::DebugNormals => false,
            ShaderCacheKeyMaterial::FullScreenQuad => false,
        }
    }

    pub fn fragment_shader_kind(&self) -> FragmentShaderKind {
        match self {
            ShaderCacheKeyMaterial::Pbr(_) => FragmentShaderKind::Pbr,
            ShaderCacheKeyMaterial::DebugNormals => FragmentShaderKind::DebugNormals,
            ShaderCacheKeyMaterial::FullScreenQuad => FragmentShaderKind::FullScreenQuad,
        }
    }

    pub fn vertex_shader_kind(&self) -> VertexShaderKind {
        match self {
            ShaderCacheKeyMaterial::Pbr(_) => VertexShaderKind::Mesh,
            ShaderCacheKeyMaterial::DebugNormals => VertexShaderKind::Mesh,
            ShaderCacheKeyMaterial::FullScreenQuad => VertexShaderKind::Quad,
        }
    }
}

#[derive(Default, Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PbrShaderCacheKeyMaterial {
    pub base_color_uv_index: Option<u32>,
    pub metallic_roughness_uv_index: Option<u32>,
    pub normal_uv_index: Option<u32>,
    pub occlusion_uv_index: Option<u32>,
    pub emissive_uv_index: Option<u32>,
    pub has_alpha_mask: bool,
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

impl ShaderCacheKey {
    pub fn into_descriptor(&self) -> Result<web_sys::GpuShaderModuleDescriptor> {
        Ok(ShaderModuleDescriptor::new(&self.into_source()?, None).into())
    }

    pub fn into_source(&self) -> Result<String> {
        let mut material = ShaderTemplateMaterial::new(self.material.has_alpha_mask());
        let mut has_normals = false;
        let mut skins = None;

        let mut vertex_input_locations = Vec::new();
        let mut sanity_check = HashSet::new();
        for attribute in &self.attributes {
            if !sanity_check.insert(std::mem::discriminant(attribute)) {
                panic!("Duplicate attribute found: {:?}", attribute);
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

        let mut push_texture = |name: &str, uv_index: u32| {
            fragment_buffer_bindings.push(DynamicBufferBinding {
                group: 2,
                index: fragment_buffer_bindings.len() as u32,
                name: format!("{name}_tex"),
                data_type: "texture_2d<f32>".to_string(),
            });

            fragment_buffer_bindings.push(DynamicBufferBinding {
                group: 2,
                index: fragment_buffer_bindings.len() as u32,
                name: format!("{name}_sampler"),
                data_type: "sampler".to_string(),
            });

            vertex_output_locations.push(VertexLocation {
                location: vertex_output_locations.len() as u32,
                interpolation: None,
                name: format!("{name}_uv"),
                data_type: "vec2<f32>".to_string(),
            });

            vertex_to_fragment_assignments.push(VertexToFragmentAssignment {
                vertex_name: format!("uv_{uv_index}"),
                fragment_name: format!("{name}_uv"),
            });
        };

        match self.material {
            ShaderCacheKeyMaterial::Pbr(material_key) => {
                if let Some(uv_index) = material_key.base_color_uv_index {
                    push_texture("base_color", uv_index);
                    material.has_base_color_tex = true;
                }

                if has_normals {
                    vertex_output_locations.push(VertexLocation {
                        location: vertex_output_locations.len() as u32,
                        interpolation: None,
                        name: "world_normal".to_string(),
                        data_type: "vec3<f32>".to_string(),
                    });
                }
            }

            ShaderCacheKeyMaterial::DebugNormals => {
                vertex_output_locations.push(VertexLocation {
                    location: vertex_output_locations.len() as u32,
                    interpolation: None,
                    name: "world_normal".to_string(),
                    data_type: "vec3<f32>".to_string(),
                });
            }

            ShaderCacheKeyMaterial::FullScreenQuad => {
            }
        };

        vertex_output_locations = vertex_output_locations
            .into_iter()
            .map(|mut loc| {
                const HARDCODED_LOCATION_LEN: u32 = 1; // account for hardcoded locations like world_position
                loc.location += HARDCODED_LOCATION_LEN;
                loc
            })
            .collect();

        let tmpl = ShaderTemplate {
            vertex_input_locations,
            vertex_output_locations,
            vertex_to_fragment_assignments,
            morphs: self.morphs,
            skins: skins.unwrap_or_default(),
            has_instance_transform: self.instancing.transform,
            vertex_shader_kind: self.material.vertex_shader_kind(),
            fragment_shader_kind: self.material.fragment_shader_kind(),
            fragment_buffer_bindings,
            material,
            has_normals,
        };

        let source = tmpl.render().unwrap();

        // tracing::info!("{:#?}", tmpl);
        // print_source(&source, false);

        Ok(source)
    }
}

#[allow(dead_code)]
fn print_source(source: &str, with_line_numbers: bool) {
    let mut output = "\n".to_string();
    let lines = source.lines();
    let mut line_number = 1;
    for line in lines {
        let formatted_line = match with_line_numbers {
            true => format!("{:>4}: {}\n", line_number, line),
            false => format!("{}\n", line),
        };
        output.push_str(&formatted_line);
        line_number += 1;
    }

    web_sys::console::log_1(&web_sys::wasm_bindgen::JsValue::from(output.as_str()));
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
    pub has_normals: bool,
    pub vertex_shader_kind: VertexShaderKind,
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

#[derive(Debug)]
pub struct ShaderTemplateMaterial {
    pub has_alpha_mask: bool,
    // the idea here is that with these gates, we can write normal shader code
    // since the variables are assigned (and from then on, we don't care about the location)
    pub has_base_color_tex: bool,
    pub has_metallic_roughness_tex: bool,
    pub has_emissive_tex: bool,
    pub has_occlusion_tex: bool,
    pub has_normal_tex: bool,
}

impl ShaderTemplateMaterial {
    pub fn new(has_alpha_mask: bool) -> Self {
        Self {
            has_alpha_mask,
            has_base_color_tex: false,
            has_metallic_roughness_tex: false,
            has_emissive_tex: false,
            has_occlusion_tex: false,
            has_normal_tex: false,
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexShaderKind {
    Mesh,
    Quad,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentShaderKind {
    DebugNormals,
    Pbr,
    FullScreenQuad,
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

new_key_type! {
    pub struct ShaderKey;
}

type Result<T> = std::result::Result<T, AwsmShaderError>;
#[derive(Error, Debug)]
pub enum AwsmShaderError {
    #[error("[shader] source error: {0}")]
    DuplicateAttribute(String),

    #[error("[shader] Compilation error: {0:?}")]
    Compilation(AwsmCoreError),
}
