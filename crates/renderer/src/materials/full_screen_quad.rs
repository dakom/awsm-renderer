use awsm_renderer_core::{bind_groups::{SamplerBindingLayout, SamplerBindingType, TextureBindingLayout}, texture::{TextureSampleType, TextureViewDimension}};

use crate::{bind_groups::material_textures::{MaterialTextureBindingEntry, MaterialTextureBindingLayoutEntry}, materials::{AwsmMaterialError, MaterialTextureCacheKey, MaterialTextureDep}, textures::Textures};
use super::Result;

pub struct FullScreenQuadMaterialDeps {
    pub scene_tex_dep: MaterialTextureDep,
}

impl FullScreenQuadMaterialDeps {
    pub fn new(scene_tex_dep: MaterialTextureDep) -> Self {
        Self {
            scene_tex_dep,
        }
    }

    pub fn cache_key(&self) -> FullScreenQuadMaterialCacheKey {
        FullScreenQuadMaterialCacheKey {
            scene_tex_cache_key: (&self.scene_tex_dep).into(),
        }
    }

    pub fn bind_group_layout_cache_key(&self) -> FullScreenQuadMaterialBindGroupLayoutCacheKey {
        FullScreenQuadMaterialBindGroupLayoutCacheKey {
        }
    }
    pub fn material(&self) -> FullScreenQuadMaterial {
        FullScreenQuadMaterial {
        }
    }

    // make sure the order matches shader.rs!
    pub  fn bind_group_layout_entries(&self) -> Vec<MaterialTextureBindingLayoutEntry> {
        let mut entries = Vec::new();

        let entry = TextureBindingLayout::new()
            .with_view_dimension(TextureViewDimension::N2d)
            .with_sample_type(TextureSampleType::Float);
        entries.push(MaterialTextureBindingLayoutEntry::Texture(entry));

        let entry =
                SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering);

        entries.push(MaterialTextureBindingLayoutEntry::Sampler(entry));

        entries
    }

    // make sure the order matches shader.rs!
    pub(super) fn bind_group_entries(
        &self,
        textures: &Textures,
    ) -> Result<Vec<MaterialTextureBindingEntry>> {
        let mut entries = Vec::new();

        let texture = textures
            .get_texture(self.scene_tex_dep.texture_key)
            .ok_or(AwsmMaterialError::MissingTexture(self.scene_tex_dep.texture_key))?;
        let texture_view = texture.create_view().map_err(|err| {
            AwsmMaterialError::CreateTextureView(format!("{:?}: {:?}", self.scene_tex_dep.texture_key, err))
        })?;
        let entry = MaterialTextureBindingEntry::Texture(texture_view);
        entries.push(entry);

        let sampler = textures
            .get_sampler(self.scene_tex_dep.sampler_key)
            .ok_or(AwsmMaterialError::MissingSampler(self.scene_tex_dep.sampler_key))?;
        let entry = MaterialTextureBindingEntry::Sampler(sampler.clone());
        entries.push(entry);

        Ok(entries)
    }
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct FullScreenQuadMaterialCacheKey {
    pub scene_tex_cache_key: MaterialTextureCacheKey,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct FullScreenQuadMaterialBindGroupLayoutCacheKey {
}

// These can be modified without regenerating any bind group bindings
// (though it will cause uniform value updates to be re-written to the GPU upon change)
#[derive(Debug, Clone)]
pub struct FullScreenQuadMaterial {
}