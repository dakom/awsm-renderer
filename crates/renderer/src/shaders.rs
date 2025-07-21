pub mod fragment;
pub mod vertex;

use std::collections::HashMap;

use askama::Template;
use awsm_renderer_core::{
    error::AwsmCoreError,
    shaders::{ShaderModuleDescriptor, ShaderModuleExt},
};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{
    shaders::{
        fragment::{cache_key::ShaderCacheKeyFragment, template::ShaderTemplateFragment},
        vertex::{ShaderCacheKeyVertex, ShaderTemplateVertex},
    },
    AwsmRenderer,
};

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

        let shader_module = self
            .gpu
            .compile_shader(&ShaderTemplate::new(&cache_key).into_descriptor()?);
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
    pub vertex: ShaderCacheKeyVertex,
    pub fragment: ShaderCacheKeyFragment,
}

impl ShaderCacheKey {
    pub fn new(vertex: ShaderCacheKeyVertex, fragment: ShaderCacheKeyFragment) -> Self {
        Self { vertex, fragment }
    }
}

#[derive(Template, Debug)]
#[template(path = "main.wgsl", whitespace = "minimize")]
struct ShaderTemplate {
    vertex: ShaderTemplateVertex,
    fragment: ShaderTemplateFragment,
}

impl ShaderTemplate {
    pub fn new(cache_key: &ShaderCacheKey) -> Self {
        let mut vertex = ShaderTemplateVertex::new(&cache_key.vertex);
        let fragment = ShaderTemplateFragment::new(&cache_key.fragment, &mut vertex);

        Self { vertex, fragment }
    }

    pub fn into_descriptor(self) -> Result<web_sys::GpuShaderModuleDescriptor> {
        Ok(ShaderModuleDescriptor::new(&self.into_source()?, None).into())
    }

    pub fn into_source(self) -> Result<String> {
        let main_source = self.render().unwrap();
        let vertex_source = self.vertex.render().unwrap();
        let fragment_source = self.fragment.render().unwrap();

        let source = format!("{main_source}\n\n{fragment_source}\n\n{vertex_source}");

        // tracing::info!("{:#?}", tmpl);
        // print_source(&fragment_source, false);

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
