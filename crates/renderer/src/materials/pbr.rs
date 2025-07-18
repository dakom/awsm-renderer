use std::collections::HashMap;

use awsm_renderer_core::{
    bind_groups::{SamplerBindingLayout, SamplerBindingType, TextureBindingLayout},
    renderer::AwsmRendererWebGpu,
    texture::{TextureSampleType, TextureViewDimension},
};

use super::{AwsmMaterialError, Result};
use crate::{
    bind_groups::{
        material_textures::{
            MaterialBindGroupKey, MaterialBindGroupLayoutKey, MaterialTextureBindingEntry,
            MaterialTextureBindingLayoutEntry,
        },
        uniform_storage::{MeshAllBindGroupBinding, UniformStorageBindGroupIndex},
        BindGroups,
    },
    buffer::dynamic_uniform::DynamicUniformBuffer,
    materials::{MaterialAlphaMode, MaterialKey},
    textures::{SamplerKey, TextureKey, Textures},
    AwsmRenderer, AwsmRendererLogging,
};

pub struct PbrMaterials {
    bind_group_layout_keys: HashMap<PbrMaterialBindGroupLayoutCacheKey, MaterialBindGroupLayoutKey>,
    bind_group_keys: HashMap<PbrMaterialBindGroupCacheKey, MaterialBindGroupKey>,
    uniform_buffer: DynamicUniformBuffer<MaterialKey>,
    uniform_buffer_gpu_dirty: bool,
}

impl Default for PbrMaterials {
    fn default() -> Self {
        Self::new()
    }
}

impl PbrMaterials {
    pub fn new() -> Self {
        Self {
            bind_group_layout_keys: HashMap::new(),
            bind_group_keys: HashMap::new(),
            uniform_buffer: DynamicUniformBuffer::new(
                PbrMaterial::INITIAL_ELEMENTS,
                PbrMaterial::BYTE_SIZE,
                PbrMaterial::UNIFORM_BUFFER_BYTE_ALIGNMENT,
                Some("PbrUniformBuffer".to_string()),
            ),
            uniform_buffer_gpu_dirty: false,
        }
    }

    pub fn buffer_offset(&self, key: MaterialKey) -> Option<usize> {
        self.uniform_buffer.offset(key)
    }

    pub fn update(&mut self, key: MaterialKey, pbr_material: &PbrMaterial) {
        self.uniform_buffer
            .update(key, &pbr_material.uniform_buffer_data());
        self.uniform_buffer_gpu_dirty = true;
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        if self.uniform_buffer_gpu_dirty {
            let _maybe_span_guard = if logging.render_timings {
                Some(tracing::span!(tracing::Level::INFO, "PBR Uniform Buffer GPU write").entered())
            } else {
                None
            };

            let bind_group_index =
                UniformStorageBindGroupIndex::MeshAll(MeshAllBindGroupBinding::PbrMaterial);
            if let Some(new_size) = self.uniform_buffer.take_gpu_needs_resize() {
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
                    self.uniform_buffer.raw_slice(),
                    None,
                    None,
                )
                .map_err(AwsmMaterialError::PbrMaterialBindGroupWrite)?;

            self.uniform_buffer_gpu_dirty = false;
        }
        Ok(())
    }
}

impl AwsmRenderer {
    pub fn add_material_pbr_bind_group_layout(
        &mut self,
        material_key: MaterialKey,
        cache_key: &PbrMaterialBindGroupLayoutCacheKey,
    ) -> Result<MaterialBindGroupLayoutKey> {
        if let Some(key) = self.materials.pbr.bind_group_layout_keys.get(cache_key) {
            // the bind group layout already exists in cache
            // but we still need to associate it with the material
            self.bind_groups
                .material_textures
                .insert_material_bind_group_layout_lookup(material_key, *key);
            return Ok(*key);
        }

        let key = self
            .bind_groups
            .material_textures
            .insert_bind_group_layout(&self.gpu, cache_key.entries())
            .map_err(AwsmMaterialError::MaterialBindGroupLayout)?;

        self.materials
            .pbr
            .bind_group_layout_keys
            .insert(cache_key.clone(), key);
        self.bind_groups
            .material_textures
            .insert_material_bind_group_layout_lookup(material_key, key);

        Ok(key)
    }

    pub fn add_material_pbr_bind_group(
        &mut self,
        material_key: MaterialKey,
        layout_key: MaterialBindGroupLayoutKey,
        cache_key: &PbrMaterialBindGroupCacheKey,
    ) -> Result<MaterialBindGroupKey> {
        if let Some(key) = self.materials.pbr.bind_group_keys.get(cache_key) {
            // the bind group already exists in cache
            // but we still need to associate it with the material
            self.bind_groups
                .material_textures
                .insert_material_bind_group_lookup(material_key, *key);
            return Ok(*key);
        }

        let entries = cache_key.entries(&self.textures)?;
        let key = self
            .bind_groups
            .material_textures
            .insert_bind_group(&self.gpu, layout_key, &entries)
            .map_err(AwsmMaterialError::MaterialBindGroup)?;

        self.materials
            .pbr
            .bind_group_keys
            .insert(cache_key.clone(), key);
        self.bind_groups
            .material_textures
            .insert_material_bind_group_lookup(material_key, key);

        Ok(key)
    }
}

#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub base_color_factor: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub emissive_factor: [f32; 3],
    // these come from initial settings which affects bind group, mesh pipeline etc.
    // so the only way to change them is to create a new material
    alpha_mode: MaterialAlphaMode,
    double_sided: bool,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            emissive_factor: [0.0, 0.0, 0.0],
            alpha_mode: MaterialAlphaMode::Opaque,
            double_sided: false,
        }
    }
}

impl PbrMaterial {
    pub const INITIAL_ELEMENTS: usize = 32; // 32 elements is a good starting point
    pub const UNIFORM_BUFFER_BYTE_ALIGNMENT: usize = 256; // minUniformBufferOffsetAlignment
    pub const BYTE_SIZE: usize = 64;

    pub fn new(alpha_mode: MaterialAlphaMode, double_sided: bool) -> Self {
        Self {
            alpha_mode,
            double_sided,
            ..Default::default()
        }
    }

    pub fn set_alpha_cutoff(&mut self, cutoff: f32) -> Result<()> {
        if let MaterialAlphaMode::Mask { .. } = self.alpha_mode {
            self.alpha_mode = MaterialAlphaMode::Mask { cutoff };
            Ok(())
        } else {
            Err(AwsmMaterialError::InvalidAlphaModeForCutoff(
                self.alpha_mode,
            ))
        }
    }

    pub fn alpha_cutoff(&self) -> Option<f32> {
        match self.alpha_mode {
            MaterialAlphaMode::Mask { cutoff } => Some(cutoff),
            _ => None,
        }
    }

    pub fn has_alpha_blend(&self) -> bool {
        matches!(self.alpha_mode, MaterialAlphaMode::Blend)
    }

    pub fn uniform_buffer_data(&self) -> [u8; Self::BYTE_SIZE] {
        let mut data = [0u8; Self::BYTE_SIZE];
        let mut offset = 0;

        enum Value {
            F32(f32),
            U32(u32),
        }

        impl From<f32> for Value {
            fn from(value: f32) -> Self {
                Value::F32(value)
            }
        }
        impl From<u32> for Value {
            fn from(value: u32) -> Self {
                Value::U32(value)
            }
        }

        let mut write = |value: Value| match value {
            Value::F32(value) => {
                let bytes = value.to_ne_bytes();
                data[offset..offset + 4].copy_from_slice(&bytes);
                offset += 4;
            }
            Value::U32(value) => {
                let bytes = value.to_ne_bytes();
                data[offset..offset + 4].copy_from_slice(&bytes);
                offset += 4;
            }
        };

        // first 16 bytes: base_color_factor: vec4<f32>
        write(self.base_color_factor[0].into());
        write(self.base_color_factor[1].into());
        write(self.base_color_factor[2].into());
        write(self.base_color_factor[3].into());

        // next 16 bytes: metallic_factor, roughness_factor, normal_scale, occlusion_strength
        write(self.metallic_factor.into());
        write(self.roughness_factor.into());
        write(self.normal_scale.into());
        write(self.occlusion_strength.into());

        // next 16 bytes: emissive factor and padding
        write(self.emissive_factor[0].into());
        write(self.emissive_factor[1].into());
        write(self.emissive_factor[2].into());
        write(0.0.into());

        // last 16 bytes: alpha_mode, alpha cutoff, double_sided, padding
        write(self.alpha_mode.variant_as_u32().into());
        write(self.alpha_cutoff().unwrap_or(0.0).into());
        write(if self.double_sided {
            1.into()
        } else {
            0.into()
        });
        write(0.0.into());

        data
    }
}

// just a cache key for re-using the bind group layouts
#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PbrMaterialBindGroupLayoutCacheKey {
    pub has_base_color_tex: bool,
    pub has_metallic_roughness_tex: bool,
    pub has_normal_tex: bool,
    pub has_occlusion_tex: bool,
    pub has_emissive_tex: bool,
}

impl From<&PbrMaterialBindGroupCacheKey> for PbrMaterialBindGroupLayoutCacheKey {
    fn from(cache_key: &PbrMaterialBindGroupCacheKey) -> Self {
        PbrMaterialBindGroupLayoutCacheKey{
            has_base_color_tex: cache_key.base_color_tex.is_some(),
            has_metallic_roughness_tex: cache_key.metallic_roughness_tex.is_some(),
            has_normal_tex: cache_key.normal_tex.is_some(),
            has_occlusion_tex: cache_key.occlusion_tex.is_some(),
            has_emissive_tex: cache_key.emissive_tex.is_some(),
        }
    }
}

impl PbrMaterialBindGroupLayoutCacheKey {
    pub fn entries(&self) -> Vec<MaterialTextureBindingLayoutEntry> {
        let mut entries = Vec::new();

        let mut push_simple = || {
            let entry = TextureBindingLayout::new()
                .with_view_dimension(TextureViewDimension::N2d)
                .with_sample_type(TextureSampleType::Float);
            entries.push(MaterialTextureBindingLayoutEntry::Texture(entry));

            let entry =
                SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering);

            entries.push(MaterialTextureBindingLayoutEntry::Sampler(entry));
        };

        if self.has_base_color_tex {
            push_simple();
        }
        if self.has_metallic_roughness_tex {
            push_simple();
        }
        if self.has_normal_tex {
            push_simple();
        }
        if self.has_occlusion_tex {
            push_simple();
        }
        if self.has_emissive_tex {
            push_simple();
        }

        entries
    }
}

// just a cache key for re-using the bind groups
#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PbrMaterialBindGroupCacheKey {
    pub base_color_tex: Option<PbrMaterialTextureCacheKey>,
    pub metallic_roughness_tex: Option<PbrMaterialTextureCacheKey>,
    pub normal_tex: Option<PbrMaterialTextureCacheKey>,
    pub occlusion_tex: Option<PbrMaterialTextureCacheKey>,
    pub emissive_tex: Option<PbrMaterialTextureCacheKey>,
}

impl PbrMaterialBindGroupCacheKey {
    pub fn entries(&self, textures: &Textures) -> Result<Vec<MaterialTextureBindingEntry>> {
        let mut entries = Vec::new();

        let mut push_texture = |dep: &PbrMaterialTextureCacheKey| -> Result<()> {
            let texture = textures
                .get_texture(dep.texture_key)
                .ok_or(AwsmMaterialError::MissingTexture(dep.texture_key))?;
            let texture_view = texture.create_view().map_err(|err| {
                AwsmMaterialError::CreateTextureView(format!("{:?}: {:?}", dep.texture_key, err))
            })?;
            let entry = MaterialTextureBindingEntry::Texture(texture_view);
            entries.push(entry);

            let sampler = textures
                .get_sampler(dep.sampler_key)
                .ok_or(AwsmMaterialError::MissingSampler(dep.sampler_key))?;
            let entry = MaterialTextureBindingEntry::Sampler(sampler.clone());
            entries.push(entry);

            Ok(())
        };

        if let Some(tex) = &self.base_color_tex {
            push_texture(tex)?;
        }

        if let Some(tex) = &self.metallic_roughness_tex {
            push_texture(tex)?;
        }
        if let Some(tex) = &self.normal_tex {
            push_texture(tex)?;
        }
        if let Some(tex) = &self.occlusion_tex {
            push_texture(tex)?;
        }
        if let Some(tex) = &self.emissive_tex {
            push_texture(tex)?;
        }

        Ok(entries)
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PbrMaterialTextureCacheKey {
    pub texture_key: TextureKey,
    pub sampler_key: SamplerKey,
}
