pub mod pbr;

use std::collections::HashMap;

use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use pbr::{PbrMaterial, PbrMaterialBindGroupLayoutCacheKey, PbrMaterialCacheKey, PbrMaterialDeps};
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::{
        material::{MaterialBindGroupLayoutKey, MaterialBindingEntry, MaterialBindingLayoutEntry},
        AwsmBindGroupError, BindGroups,
    },
    shaders::ShaderCacheKeyMaterial,
    textures::{SamplerKey, TextureKey, Textures},
};

pub struct Materials {
    materials: SlotMap<MaterialKey, Material>,
    cache: HashMap<MaterialCacheKey, MaterialKey>,
    bind_group_layout_cache: HashMap<MaterialBindGroupLayoutCacheKey, MaterialBindGroupLayoutKey>,
}

// The final material type with adjustable properties
pub enum Material {
    Pbr(PbrMaterial),
}

// The original dependencies, with textures etc.
pub enum MaterialDeps {
    Pbr(PbrMaterialDeps),
}

// an internal key to hash ad-hoc material generation
// this is not the same as the material key
// it's used to prevent duplicate materials
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
enum MaterialCacheKey {
    Pbr(PbrMaterialCacheKey),
}

// This is an internal cache optimization so that we
// can reuse the same bind group layout for multiple materials
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
enum MaterialBindGroupLayoutCacheKey {
    Pbr(PbrMaterialBindGroupLayoutCacheKey),
}

pub struct MaterialTextureDep {
    pub texture_key: TextureKey,
    pub sampler_key: SamplerKey,
    pub uv_index: usize,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
struct MaterialTextureCacheKey {
    texture_key: TextureKey,
    sampler_key: SamplerKey,
}

impl From<&MaterialTextureDep> for MaterialTextureCacheKey {
    fn from(dep: &MaterialTextureDep) -> Self {
        Self {
            texture_key: dep.texture_key,
            sampler_key: dep.sampler_key,
        }
    }
}

impl MaterialDeps {
    pub fn shader_cache_key(&self) -> ShaderCacheKeyMaterial {
        match self {
            Self::Pbr(deps) => ShaderCacheKeyMaterial::Pbr(deps.shader_cache_key()),
        }
    }

    fn cache_key(&self) -> MaterialCacheKey {
        match self {
            Self::Pbr(deps) => MaterialCacheKey::Pbr(deps.cache_key()),
        }
    }

    fn bind_group_layout_cache_key(&self) -> MaterialBindGroupLayoutCacheKey {
        match self {
            Self::Pbr(deps) => {
                MaterialBindGroupLayoutCacheKey::Pbr(deps.bind_group_layout_cache_key())
            }
        }
    }

    pub fn material(&self) -> Material {
        match self {
            Self::Pbr(deps) => Material::Pbr(deps.material()),
        }
    }

    fn bind_group_layout_entries(&self) -> Vec<MaterialBindingLayoutEntry> {
        match self {
            Self::Pbr(deps) => deps.bind_group_layout_entries(),
        }
    }

    fn bind_group_entries(&self, textures: &Textures) -> Result<Vec<MaterialBindingEntry>> {
        match self {
            Self::Pbr(deps) => deps.bind_group_entries(textures),
        }
    }
}

impl Materials {
    pub fn new() -> Self {
        Self {
            materials: SlotMap::with_key(),
            cache: HashMap::new(),
            bind_group_layout_cache: HashMap::new(),
        }
    }

    pub fn get_or_insert(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
        textures: &Textures,
        deps: MaterialDeps,
    ) -> Result<MaterialKey> {
        let cache_key = deps.cache_key();

        // Try to get the material from the cache first
        if let Some(material_key) = self.cache.get(&cache_key) {
            return Ok(*material_key);
        }

        // nope, but maybe we at least have the layout cached
        let bind_group_layout_cache_key = deps.bind_group_layout_cache_key();
        let bind_group_layout_key = match self
            .bind_group_layout_cache
            .get(&bind_group_layout_cache_key)
        {
            Some(key) => *key,
            None => {
                // nope, create the layout
                let entries = deps.bind_group_layout_entries();
                let bind_group_layout_key = bind_groups
                    .materials
                    .insert_layout(&gpu, entries)
                    .map_err(AwsmMaterialError::MaterialBindGroupLayout)?;

                self.bind_group_layout_cache
                    .insert(bind_group_layout_cache_key, bind_group_layout_key);

                bind_group_layout_key
            }
        };

        let material_key = self.materials.insert(deps.material());

        bind_groups
            .materials
            .insert_material(
                &gpu,
                material_key,
                bind_group_layout_key,
                &deps.bind_group_entries(textures)?,
            )
            .map_err(AwsmMaterialError::MaterialBindGroup)?;

        Ok(material_key)
    }
}

new_key_type! {
    pub struct MaterialKey;
}

type Result<T> = std::result::Result<T, AwsmMaterialError>;

#[derive(Error, Debug)]
pub enum AwsmMaterialError {
    #[error("[material] unable to create bind group: {0:?}")]
    MaterialBindGroup(AwsmBindGroupError),

    #[error("[material] unable to create bind group layout: {0:?}")]
    MaterialBindGroupLayout(AwsmBindGroupError),

    #[error("[material] missing texture: {0:?}")]
    MissingTexture(TextureKey),

    #[error("[material] missing sampler: {0:?}")]
    MissingSampler(SamplerKey),

    #[error("[material] create texture view: {0}")]
    CreateTextureView(String),
}
