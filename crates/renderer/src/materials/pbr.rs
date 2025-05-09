use awsm_renderer_core::{
    bind_groups::{SamplerBindingLayout, SamplerBindingType, TextureBindingLayout},
    texture::{TextureSampleType, TextureViewDimension},
};

use crate::{
    bind_groups::material_textures::{
        MaterialTextureBindingEntry, MaterialTextureBindingLayoutEntry,
    },
    shaders::PbrShaderCacheKeyMaterial,
    textures::Textures,
};

use super::{
    AwsmMaterialError, MaterialAlphaMode, MaterialTextureCacheKey, MaterialTextureDep, Result,
};

// These can be modified without regenerating any bind group bindings
// (though it will cause uniform value updates to be re-written to the GPU upon change)
#[derive(Debug, Clone)]
pub struct PbrMaterial {
    pub base_color_factor: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub emissive_factor: [f32; 3],
    pub alpha_mode: MaterialAlphaMode,
    pub double_sided: bool,
}

impl PbrMaterial {
    pub const INITIAL_ELEMENTS: usize = 32; // 32 elements is a good starting point
    pub const UNIFORM_BUFFER_BYTE_ALIGNMENT: usize = 256; // minUniformBufferOffsetAlignment
    pub const BYTE_SIZE: usize = 64;

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
        write(self.alpha_mode.cutoff().into());
        write(if self.double_sided {
            1.into()
        } else {
            0.into()
        });
        write(0.0.into());

        data
    }
}

pub struct PbrMaterialDeps {
    pub base_color_tex: Option<MaterialTextureDep>,
    pub base_color_factor: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub metallic_roughness_tex: Option<MaterialTextureDep>,
    pub normal_tex: Option<MaterialTextureDep>,
    pub normal_scale: f32,
    pub occlusion_tex: Option<MaterialTextureDep>,
    pub occlusion_strength: f32,
    pub emissive_tex: Option<MaterialTextureDep>,
    pub emissive_factor: [f32; 3],
    pub alpha_mode: MaterialAlphaMode,
    pub double_sided: bool,
}

impl Default for PbrMaterialDeps {
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            base_color_tex: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_tex: None,
            normal_tex: None,
            normal_scale: 1.0,
            occlusion_tex: None,
            occlusion_strength: 1.0,
            emissive_tex: None,
            emissive_factor: [0.0, 0.0, 0.0],
            alpha_mode: MaterialAlphaMode::Opaque,
            double_sided: false,
        }
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct PbrMaterialCacheKey {
    pub base_color_tex: Option<MaterialTextureCacheKey>,
    pub metallic_roughness_tex: Option<MaterialTextureCacheKey>,
    pub normal_tex: Option<MaterialTextureCacheKey>,
    pub occlusion_tex: Option<MaterialTextureCacheKey>,
    pub emissive_tex: Option<MaterialTextureCacheKey>,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub struct PbrMaterialBindGroupLayoutCacheKey {
    pub base_color_tex: bool,
    pub metallic_roughness_tex: bool,
    pub normal_tex: bool,
    pub occlusion_tex: bool,
    pub emissive_tex: bool,
}

impl PbrMaterialDeps {
    pub(super) fn cache_key(&self) -> PbrMaterialCacheKey {
        PbrMaterialCacheKey {
            base_color_tex: self
                .base_color_tex
                .as_ref()
                .map(MaterialTextureCacheKey::from),
            metallic_roughness_tex: self
                .metallic_roughness_tex
                .as_ref()
                .map(MaterialTextureCacheKey::from),
            normal_tex: self.normal_tex.as_ref().map(MaterialTextureCacheKey::from),
            occlusion_tex: self
                .occlusion_tex
                .as_ref()
                .map(MaterialTextureCacheKey::from),
            emissive_tex: self
                .emissive_tex
                .as_ref()
                .map(MaterialTextureCacheKey::from),
        }
    }
    pub(super) fn bind_group_layout_cache_key(&self) -> PbrMaterialBindGroupLayoutCacheKey {
        PbrMaterialBindGroupLayoutCacheKey {
            base_color_tex: self.base_color_tex.is_some(),
            metallic_roughness_tex: self.metallic_roughness_tex.is_some(),
            normal_tex: self.normal_tex.is_some(),
            occlusion_tex: self.occlusion_tex.is_some(),
            emissive_tex: self.emissive_tex.is_some(),
        }
    }

    pub fn material(&self) -> PbrMaterial {
        PbrMaterial {
            base_color_factor: self.base_color_factor,
            metallic_factor: self.metallic_factor,
            roughness_factor: self.roughness_factor,
            normal_scale: self.normal_scale,
            occlusion_strength: self.occlusion_strength,
            emissive_factor: self.emissive_factor,
            alpha_mode: self.alpha_mode,
            double_sided: self.double_sided,
        }
    }

    pub fn shader_cache_key(&self) -> PbrShaderCacheKeyMaterial {
        PbrShaderCacheKeyMaterial {
            base_color_uv_index: self.base_color_tex.as_ref().map(|dep| dep.uv_index as u32),
        }
    }

    // make sure the order matches shader.rs!
    pub(super) fn bind_group_layout_entries(&self) -> Vec<MaterialTextureBindingLayoutEntry> {
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

        if self.base_color_tex.is_some() {
            push_simple();
        }
        if self.metallic_roughness_tex.is_some() {
            push_simple();
        }
        if self.normal_tex.is_some() {
            push_simple();
        }
        if self.occlusion_tex.is_some() {
            push_simple();
        }
        if self.emissive_tex.is_some() {
            push_simple();
        }

        entries
    }

    // make sure the order matches shader.rs!
    pub(super) fn bind_group_entries(
        &self,
        textures: &Textures,
    ) -> Result<Vec<MaterialTextureBindingEntry>> {
        let mut entries = Vec::new();

        let mut push_texture = |dep: &MaterialTextureDep| -> Result<()> {
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
