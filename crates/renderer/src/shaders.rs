pub mod mesh;
pub mod pbr;
pub mod post_process;

use core::panic;
use std::collections::HashMap;

use askama::Template;
use awsm_renderer_core::{
    error::AwsmCoreError,
    shaders::{ShaderModuleDescriptor, ShaderModuleExt},
};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{shaders::{mesh::{MeshShaderCacheKeyGeometry, MeshShaderTemplateGeometry}, pbr::{PbrShaderCacheKeyMaterial, PbrShaderTemplateMaterial}, post_process::{PostProcessShaderCacheKeyMaterial, PostProcessShaderTemplateMaterial}}, AwsmRenderer};

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

        let shader_module = self.gpu.compile_shader(&cache_key.clone().into_descriptor()?);
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
    pub material: ShaderCacheKeyMaterial,
    pub geometry: ShaderCacheKeyGeometry,
}


impl ShaderCacheKey {
    pub fn new(geometry: ShaderCacheKeyGeometry, material: ShaderCacheKeyMaterial) -> Self {
        Self {
            geometry,
            material,
        }
    }

    pub fn with_geometry(mut self, geometry: ShaderCacheKeyGeometry) -> Self {
        self.geometry = geometry;
        self
    }

    pub fn with_material(mut self, material: ShaderCacheKeyMaterial) -> Self {
        self.material = material;
        self
    }
}



#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum ShaderCacheKeyGeometry {
    Mesh(MeshShaderCacheKeyGeometry),
    Quad
}

impl ShaderCacheKeyGeometry {
    pub fn as_mesh(&self) -> &MeshShaderCacheKeyGeometry {
        match self {
            ShaderCacheKeyGeometry::Mesh(mesh_geometry) => &mesh_geometry,
            ShaderCacheKeyGeometry::Quad => panic!("Cannot convert Quad to MeshShaderCacheKeyGeometry"),
        }
    }
}

impl From<ShaderCacheKeyGeometry> for  ShaderTemplateGeometry {
    fn from(geometry: ShaderCacheKeyGeometry) -> Self {
        match geometry {
            ShaderCacheKeyGeometry::Mesh(mesh_geometry) => ShaderTemplateGeometry::Mesh(mesh_geometry.into()),
            ShaderCacheKeyGeometry::Quad => ShaderTemplateGeometry::Quad,
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderCacheKeyMaterial {
    Pbr(PbrShaderCacheKeyMaterial),
    PostProcess(PostProcessShaderCacheKeyMaterial),
    DebugNormals,
}

impl ShaderCacheKeyMaterial {
    pub fn has_alpha_mask(&self) -> bool {
        match self {
            ShaderCacheKeyMaterial::Pbr(material_key) => material_key.has_alpha_mask,
            ShaderCacheKeyMaterial::DebugNormals => false,
            ShaderCacheKeyMaterial::PostProcess(_) => false,
        }
    }

    pub fn fragment_shader_kind(&self) -> FragmentShaderKind {
        match self {
            ShaderCacheKeyMaterial::Pbr(_) => FragmentShaderKind::Pbr,
            ShaderCacheKeyMaterial::DebugNormals => FragmentShaderKind::DebugNormals,
            ShaderCacheKeyMaterial::PostProcess(_) => FragmentShaderKind::PostProcess,
        }
    }

    pub fn vertex_shader_kind(&self) -> VertexShaderKind {
        match self {
            ShaderCacheKeyMaterial::Pbr(_) => VertexShaderKind::Mesh,
            ShaderCacheKeyMaterial::DebugNormals => VertexShaderKind::Mesh,
            ShaderCacheKeyMaterial::PostProcess(_) => VertexShaderKind::Quad,
        }
    }

    pub fn into_template(self, geometry: &ShaderTemplateGeometry) -> ShaderTemplateMaterial {
        match self {
            ShaderCacheKeyMaterial::Pbr(cache_key) => {
                ShaderTemplateMaterial::Pbr(match geometry {
                    ShaderTemplateGeometry::Mesh(mesh_geometry) => {
                        cache_key.into_template(mesh_geometry.has_normals)
                    },
                    ShaderTemplateGeometry::Quad => {
                        cache_key.into_template(false)
                    }
                })
            }
            ShaderCacheKeyMaterial::DebugNormals => {
                ShaderTemplateMaterial::Pbr(PbrShaderCacheKeyMaterial::default().into_template(true))
            }
            ShaderCacheKeyMaterial::PostProcess(cache_key) => {
                ShaderTemplateMaterial::PostProcess(cache_key.into())
            }
        }
    }
}

impl ShaderCacheKey {
    pub fn into_descriptor(self) -> Result<web_sys::GpuShaderModuleDescriptor> {
        Ok(ShaderModuleDescriptor::new(&self.into_source()?, None).into())
    }

    pub fn into_source(self) -> Result<String> {
        let geometry:ShaderTemplateGeometry = self.geometry.into();
        let material:ShaderTemplateMaterial = self.material.into_template(&geometry);

        let tmpl = ShaderTemplate {
            vertex_shader_kind: self.material.vertex_shader_kind(),
            fragment_shader_kind: self.material.fragment_shader_kind(),
            material,
            geometry
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
            true => format!("{line_number:>4}: {line}\n"),
            false => format!("{line}\n"),
        };
        output.push_str(&formatted_line);
        line_number += 1;
    }

    web_sys::console::log_1(&web_sys::wasm_bindgen::JsValue::from(output.as_str()));
}

#[derive(Template, Debug)]
#[template(path = "main.wgsl", whitespace = "minimize")]
struct ShaderTemplate  {
    pub vertex_shader_kind: VertexShaderKind,
    pub fragment_shader_kind: FragmentShaderKind,
    pub material: ShaderTemplateMaterial,
    pub geometry: ShaderTemplateGeometry,
}

#[derive(Debug)]
pub enum ShaderTemplateMaterial {
    Pbr(PbrShaderTemplateMaterial),
    PostProcess(PostProcessShaderTemplateMaterial),
}


impl ShaderTemplateMaterial {
    pub fn as_pbr(&self) -> &PbrShaderTemplateMaterial {
        match self {
            ShaderTemplateMaterial::Pbr(material) => material,
            ShaderTemplateMaterial::PostProcess(_) => {
                panic!("Cannot convert PostProcessShaderTemplateMaterial to PbrShaderTemplateMaterial");
            },
        }
    }

    pub fn as_pbr_mut(&mut self) -> &mut PbrShaderTemplateMaterial {
        match self {
            ShaderTemplateMaterial::Pbr(material) => material,
            ShaderTemplateMaterial::PostProcess(_) => {
                panic!("Cannot convert PostProcessShaderTemplateMaterial to PbrShaderTemplateMaterial");
            },
        }
    }

    pub fn as_post_process(&self) -> &PostProcessShaderTemplateMaterial {
        match self {
            ShaderTemplateMaterial::PostProcess(material) => material,
            ShaderTemplateMaterial::Pbr(_) => {
                panic!("Cannot convert PbrShaderTemplateMaterial to PostProcessShaderTemplateMaterial");
            },
        }
    }
}

#[derive(Debug)]
pub enum ShaderTemplateGeometry {
    Mesh(MeshShaderTemplateGeometry),
    Quad
}

impl ShaderTemplateGeometry {
    pub fn as_mesh(&self) -> &MeshShaderTemplateGeometry {
        match self {
            ShaderTemplateGeometry::Mesh(geometry) => geometry,
            ShaderTemplateGeometry::Quad => {
                panic!("Cannot convert Quad to MeshShaderTemplateGeometry");
            },
        }
    }

    pub fn as_quad(&self) -> &MeshShaderTemplateGeometry {
        match self {
            ShaderTemplateGeometry::Quad => {
                panic!("Cannot convert MeshShaderTemplateGeometry to Quad");
            },
            ShaderTemplateGeometry::Mesh(geometry) => geometry,
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
    PostProcess,
}

#[derive(Debug)]
pub struct VertexLocation {
    location: u32,
    interpolation: Option<&'static str>,
    name: String,
    data_type: String,
}

#[derive(Debug)]
pub struct DynamicBufferBinding {
    group: u32,
    index: u32,
    name: String,
    data_type: String,
}

#[derive(Debug)]
pub struct VertexToFragmentAssignment {
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
