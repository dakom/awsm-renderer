use awsm_renderer_core::{
    bind_groups::{SamplerBindingLayout, SamplerBindingType, TextureBindingLayout},
    texture::{TextureSampleType, TextureViewDimension},
};

use super::{AwsmMaterialError, Result};
use crate::{
    bind_groups::material_textures::{
        MaterialBindGroupKey, MaterialBindGroupLayoutKey, MaterialTextureBindingEntry,
        MaterialTextureBindingLayoutEntry,
    },
    materials::MaterialKey,
    AwsmRenderer,
};

pub struct PostProcessMaterials {
    cached_bind_group_layout_key: Option<MaterialBindGroupLayoutKey>,
}

impl Default for PostProcessMaterials {
    fn default() -> Self {
        Self::new()
    }
}

impl PostProcessMaterials {
    pub fn new() -> Self {
        PostProcessMaterials {
            cached_bind_group_layout_key: None,
        }
    }
    pub fn update(&mut self, _material_key: MaterialKey, _material: &PostProcessMaterial) {
        // nothing to do here until we need uniforms etc.
    }
}

impl AwsmRenderer {
    pub fn add_material_post_process_bind_group_layout(
        &mut self,
        material_key: MaterialKey,
    ) -> Result<MaterialBindGroupLayoutKey> {
        if let Some(key) = self.materials.post_process.cached_bind_group_layout_key {
            // the bind group layout already exists in cache
            return Ok(key);
        }

        let texture_entry = TextureBindingLayout::new()
            .with_view_dimension(TextureViewDimension::N2d)
            .with_sample_type(TextureSampleType::Float);

        let sampler_entry =
            SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering);

        let key = self
            .bind_groups
            .material_textures
            .insert_bind_group_layout(
                &self.gpu,
                vec![
                    MaterialTextureBindingLayoutEntry::Texture(texture_entry),
                    MaterialTextureBindingLayoutEntry::Sampler(sampler_entry),
                ],
            )
            .map_err(AwsmMaterialError::MaterialBindGroupLayout)?;

        self.materials.post_process.cached_bind_group_layout_key = Some(key);
        self.bind_groups
            .material_textures
            .insert_material_bind_group_layout_lookup(material_key, key);

        Ok(key)
    }

    // this doesn't use a cache - it is always created anew, the caching is based on screen resizing
    pub fn add_material_post_process_bind_group(
        &mut self,
        material_key: MaterialKey,
        scene_texture_view: web_sys::GpuTextureView,
        scene_texture_sampler: web_sys::GpuSampler,
    ) -> Result<MaterialBindGroupKey> {
        // this will be retrieved from the cache
        let layout_key = self.add_material_post_process_bind_group_layout(material_key)?;

        let key = self
            .bind_groups
            .material_textures
            .insert_bind_group(
                &self.gpu,
                layout_key,
                &[MaterialTextureBindingEntry::Texture(scene_texture_view),
                    MaterialTextureBindingEntry::Sampler(scene_texture_sampler)],
            )
            .map_err(AwsmMaterialError::MaterialBindGroup)?;

        self.bind_groups
            .material_textures
            .insert_material_bind_group_lookup(material_key, key);

        Ok(key)
    }
}

#[derive(Debug, Clone)]
pub struct PostProcessMaterial {}

impl Default for PostProcessMaterial {
    fn default() -> Self {
        Self::new()
    }
}

impl PostProcessMaterial {
    pub fn new() -> Self {
        PostProcessMaterial {}
    }
}
