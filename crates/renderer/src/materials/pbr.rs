use awsm_renderer_core::{bind_groups::{SamplerBindingLayout, SamplerBindingType, TextureBindingLayout}, texture::{TextureSampleType, TextureViewDimension}};

use crate::{bind_groups::material::{MaterialBindingEntry, MaterialBindingLayoutEntry}, shaders::PbrShaderCacheKeyMaterial, textures::Textures};

use super::{MaterialTextureCacheKey, MaterialTextureDep, Result, AwsmMaterialError};

pub struct PbrMaterial {
}

#[derive(Default)]
pub struct PbrMaterialDeps {
    pub base_color: Option<MaterialTextureDep>,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct PbrMaterialCacheKey {
    pub base_color: Option<MaterialTextureCacheKey>,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub struct PbrMaterialBindGroupLayoutCacheKey {
    pub base_color: bool,
}

impl PbrMaterialDeps {
    pub(super) fn cache_key(&self) -> PbrMaterialCacheKey {
        let mut key = PbrMaterialCacheKey::default();
        key.base_color = self.base_color.as_ref().map(MaterialTextureCacheKey::from);

        key
    }
    pub(super) fn bind_group_layout_cache_key(&self) -> PbrMaterialBindGroupLayoutCacheKey {
        let mut key = PbrMaterialBindGroupLayoutCacheKey::default();
        key.base_color = self.base_color.is_some();

        key
    }

    pub fn material(&self) -> PbrMaterial {
        PbrMaterial {
        }
    }

    pub fn shader_cache_key(&self) -> PbrShaderCacheKeyMaterial {
        PbrShaderCacheKeyMaterial {
            base_color_uv_index: self.base_color.as_ref().map(|dep| dep.uv_index as u32)
        }
    }

    // make sure the order matches shader.rs!
    pub(super) fn bind_group_layout_entries(&self) -> Vec<MaterialBindingLayoutEntry> {
        let mut entries = Vec::new();

        if self.base_color.is_some() {
            let entry = TextureBindingLayout::new()
                .with_view_dimension(TextureViewDimension::N2d)
                .with_sample_type(TextureSampleType::Float);
            entries.push(MaterialBindingLayoutEntry::Texture(entry));

            let entry =
                SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering);

            entries.push(MaterialBindingLayoutEntry::Sampler(entry));
        }

        entries
    }

    // make sure the order matches shader.rs!
    pub(super) fn bind_group_entries(&self, textures: &Textures) -> Result<Vec<MaterialBindingEntry>> {
        let mut entries = Vec::new();

        let mut push_texture = |dep: &MaterialTextureDep| -> Result<()> {
            let texture = textures.get_texture(dep.texture_key).ok_or(AwsmMaterialError::MissingTexture(dep.texture_key))?;
            let texture_view = texture.create_view().map_err(|err| AwsmMaterialError::CreateTextureView(format!("{:?}: {:?}", dep.texture_key, err)))?;
            let entry = MaterialBindingEntry::Texture(texture_view);
            entries.push(entry);

            let sampler = textures.get_sampler(dep.sampler_key).ok_or(AwsmMaterialError::MissingSampler(dep.sampler_key))?;
            let entry = MaterialBindingEntry::Sampler(sampler.clone());
            entries.push(entry);

            Ok(())
        };

        if let Some(dep) = &self.base_color {
            push_texture(dep)?;
        }

        Ok(entries)
    }
}