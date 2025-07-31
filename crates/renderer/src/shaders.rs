use std::collections::HashMap;

use awsm_renderer_core::{error::AwsmCoreError, shaders::ShaderModuleDescriptor};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use awsm_renderer_core::shaders::ShaderModuleExt;

use crate::{render_passes::{shader_cache_key::ShaderCacheKeyRenderPass, shader_template::ShaderTemplateRenderPass}, AwsmRenderer};

impl AwsmRenderer {
    pub async fn add_shader(&mut self, cache_key: impl Into<ShaderCacheKey>) -> Result<ShaderKey> {
        let cache_key: ShaderCacheKey = cache_key.into();
        if let Some(shader_key) = self.shaders.get_shader_key_from_cache(&cache_key) {
            return Ok(shader_key);
        }

        let shader_module = self
            .gpu
            .compile_shader(&ShaderTemplate::try_from(&cache_key)?.into_descriptor()?);

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

pub struct Shaders {
    lookup: SlotMap<ShaderKey, web_sys::GpuShaderModule>,
    cache: HashMap<ShaderCacheKey, ShaderKey>,
    reverse_cache: HashMap<ShaderKey, ShaderCacheKey>,
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

impl Default for Shaders {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum ShaderCacheKey {
    RenderPass(ShaderCacheKeyRenderPass),
}


pub enum ShaderTemplate {
    RenderPass(ShaderTemplateRenderPass),
}

impl TryFrom<&ShaderCacheKey> for ShaderTemplate {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKey) -> Result<Self> {
        match value {
            ShaderCacheKey::RenderPass(cache_key) => Ok(ShaderTemplate::RenderPass(cache_key.try_into()?)),
        }
    }
}

impl ShaderTemplate {
    pub fn into_descriptor(self) -> Result<web_sys::GpuShaderModuleDescriptor> {
        Ok(ShaderModuleDescriptor::new(&self.into_source()?, None).into())
    }

    pub fn into_source(self) -> Result<String> {
        let source = match self {
            ShaderTemplate::RenderPass(tmpl) => tmpl.into_source()?,
        };
        //tracing::info!("{:#?}", tmpl);
        // print_shader_source(&source, true);

        Ok(source)
    }
}

#[allow(dead_code)]
pub fn print_shader_source(source: &str, with_line_numbers: bool) {
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

pub type Result<T> = std::result::Result<T, AwsmShaderError>;
#[derive(Error, Debug)]
pub enum AwsmShaderError {
    #[error("[shader] source error: {0}")]
    DuplicateAttribute(String),

    #[error("[shader] Compilation error: {0:?}")]
    Compilation(AwsmCoreError),

    #[error("[shader] Template error: {0:?}")]
    Template(#[from] askama::Error),
}
