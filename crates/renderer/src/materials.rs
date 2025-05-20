pub mod pbr;

use std::collections::HashMap;

use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use pbr::{PbrMaterial, PbrMaterialBindGroupLayoutCacheKey, PbrMaterialCacheKey, PbrMaterialDeps};
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::{
        material_textures::{
            MaterialBindGroupLayoutKey, MaterialTextureBindingEntry,
            MaterialTextureBindingLayoutEntry,
        },
        uniform_storage::{MeshAllBindGroupBinding, UniformStorageBindGroupIndex},
        AwsmBindGroupError, BindGroups,
    },
    buffer::dynamic_uniform::DynamicUniformBuffer,
    shaders::ShaderCacheKeyMaterial,
    textures::{SamplerKey, TextureKey, Textures},
    AwsmRendererLogging,
};

pub struct Materials {
    materials: SlotMap<MaterialKey, Material>,
    material_alpha: SecondaryMap<MaterialKey, bool>,
    cache: HashMap<MaterialCacheKey, MaterialKey>,
    bind_group_layout_cache: HashMap<MaterialBindGroupLayoutCacheKey, MaterialBindGroupLayoutKey>,
    pbr_uniform_buffer: DynamicUniformBuffer<MaterialKey>,
    pbr_uniform_buffer_gpu_dirty: bool,
}

// The material type with adjustable properties
#[derive(Debug, Clone)]
pub enum Material {
    Pbr(PbrMaterial),
}

impl Material {
    pub fn has_alpha(&self) -> bool {
        match self {
            Self::Pbr(pbr_material) => pbr_material.alpha_mode == MaterialAlphaMode::Blend,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterialAlphaMode {
    Opaque,
    Mask { cutoff: f32 },
    Blend,
}

impl MaterialAlphaMode {
    pub fn variant_as_u32(&self) -> u32 {
        match self {
            Self::Opaque => 0,
            Self::Mask { .. } => 1,
            Self::Blend => 2,
        }
    }

    pub fn cutoff(&self) -> f32 {
        match self {
            Self::Opaque => 0.0,
            Self::Mask { cutoff } => *cutoff,
            Self::Blend => 0.0,
        }
    }
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

    fn bind_group_layout_entries(&self) -> Vec<MaterialTextureBindingLayoutEntry> {
        match self {
            Self::Pbr(deps) => deps.bind_group_layout_entries(),
        }
    }

    fn bind_group_entries(&self, textures: &Textures) -> Result<Vec<MaterialTextureBindingEntry>> {
        match self {
            Self::Pbr(deps) => deps.bind_group_entries(textures),
        }
    }
}

impl Default for Materials {
    fn default() -> Self {
        Self::new()
    }
}

impl Materials {
    pub fn new() -> Self {
        Self {
            materials: SlotMap::with_key(),
            material_alpha: SecondaryMap::new(),
            cache: HashMap::new(),
            bind_group_layout_cache: HashMap::new(),
            pbr_uniform_buffer: DynamicUniformBuffer::new(
                PbrMaterial::INITIAL_ELEMENTS,
                PbrMaterial::BYTE_SIZE,
                PbrMaterial::UNIFORM_BUFFER_BYTE_ALIGNMENT,
                Some("PbrUniformBuffer".to_string()),
            ),
            pbr_uniform_buffer_gpu_dirty: false,
        }
    }

    pub fn pbr_buffer_offset(&self, key: MaterialKey) -> Option<usize> {
        self.pbr_uniform_buffer.offset(key)
    }

    // will internally use a cache to re-use the same material and/or layout
    pub fn insert(
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
                    .material_textures
                    .insert_bind_group_layout(gpu, entries)
                    .map_err(AwsmMaterialError::MaterialBindGroupLayout)?;

                self.bind_group_layout_cache
                    .insert(bind_group_layout_cache_key, bind_group_layout_key);

                bind_group_layout_key
            }
        };

        let material = deps.material();

        let material_key = self.materials.insert(material.clone());

        self.material_alpha
            .insert(material_key, material.has_alpha());

        bind_groups
            .material_textures
            .insert_material_texture(
                gpu,
                material_key,
                bind_group_layout_key,
                &deps.bind_group_entries(textures)?,
            )
            .map_err(AwsmMaterialError::MaterialBindGroup)?;

        #[allow(irrefutable_let_patterns)]
        if let Material::Pbr(pbr_material) = &material {
            self.pbr_uniform_buffer
                .update(material_key, &pbr_material.uniform_buffer_data());
            self.pbr_uniform_buffer_gpu_dirty = true;
        }

        Ok(material_key)
    }

    pub fn update(&mut self, key: MaterialKey, mut f: impl FnMut(&mut Material)) {
        if let Some(material) = self.materials.get_mut(key) {
            let old_has_alpha = material.has_alpha();
            f(material);
            let new_has_alpha = material.has_alpha();
            if old_has_alpha != new_has_alpha {
                self.material_alpha.insert(key, new_has_alpha);
            }
            match material {
                Material::Pbr(pbr_material) => {
                    self.pbr_uniform_buffer
                        .update(key, &pbr_material.uniform_buffer_data());
                    self.pbr_uniform_buffer_gpu_dirty = true;
                }
            }
        }
    }

    pub fn has_alpha(&self, key: MaterialKey) -> Option<bool> {
        self.material_alpha.get(key).copied()
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.pbr_uniform_buffer_gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "PBR Uniform Buffer GPU write").entered())
            } else {
                None
            };

            let bind_group_index =
                UniformStorageBindGroupIndex::MeshAll(MeshAllBindGroupBinding::PbrMaterial);
            if let Some(new_size) = self.pbr_uniform_buffer.take_gpu_needs_resize() {
                bind_groups
                    .uniform_storages
                    .gpu_resize(gpu, bind_group_index, new_size)
                    .map_err(AwsmMaterialError::PbrMaterialBindGroupResize)?;
            }

            bind_groups
                .uniform_storages
                .gpu_write(
                    gpu,
                    bind_group_index,
                    None,
                    self.pbr_uniform_buffer.raw_slice(),
                    None,
                    None,
                )
                .map_err(AwsmMaterialError::PbrMaterialBindGroupWrite)?;

            self.pbr_uniform_buffer_gpu_dirty = false;
        }
        Ok(())
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

    #[error("[material] pbr unable to resize bind group: {0:?}")]
    PbrMaterialBindGroupResize(AwsmBindGroupError),

    #[error("[material] pbr unable to write bind group: {0:?}")]
    PbrMaterialBindGroupWrite(AwsmBindGroupError),
}
