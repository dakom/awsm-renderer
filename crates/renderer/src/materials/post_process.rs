use awsm_renderer_core::{
    bind_groups::{SamplerBindingLayout, SamplerBindingType, TextureBindingLayout},
    sampler::SamplerDescriptor,
    texture::{TextureSampleType, TextureViewDimension},
};

use super::{AwsmMaterialError, Result};
use crate::{
    bind_groups::material_textures::{
        MaterialBindGroupKey, MaterialBindGroupLayoutKey, MaterialTextureBindingEntry,
        MaterialTextureBindingLayoutEntry,
    }, materials::MaterialKey, render::textures::RenderTextureViews, textures::SamplerKey, AwsmRenderer
};

pub struct PostProcessMaterials {
    cached_bind_group_layout_key: Option<MaterialBindGroupLayoutKey>,
    cached_scene_sampler_key: Option<SamplerKey>,
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
            cached_scene_sampler_key: None,
        }
    }
    pub fn update(&mut self, _material_key: MaterialKey, _material: &PostProcessMaterial) {
        // nothing to do here until we need uniforms etc.
    }
}

impl AwsmRenderer {
    pub fn add_material_post_process_scene_sampler(
        &mut self,
        sampler_descriptor: SamplerDescriptor,
    ) -> Result<SamplerKey> {
        if let Some(sampler_key) = self.materials.post_process.cached_scene_sampler_key {
            // the sampler already exists in cache
            return Ok(sampler_key);
        }

        let sampler = self.gpu.create_sampler(Some(&sampler_descriptor.into()));
        let sampler_key = self.textures.add_sampler(sampler);
        self.materials.post_process.cached_scene_sampler_key = Some(sampler_key);
        Ok(sampler_key)
    }

    pub fn add_material_post_process_bind_group_layout(
        &mut self,
        // we don't need a cache key since all post-process materials share the same layout
        material_key: MaterialKey,
    ) -> Result<MaterialBindGroupLayoutKey> {
        if let Some(key) = self.materials.post_process.cached_bind_group_layout_key {
            // the bind group layout already exists in cache, though we still need to associate it with the material key
            self.bind_groups
                .material_textures
                .insert_material_bind_group_layout_lookup(material_key, key);
            return Ok(key);
        }

        let scene_texture_entry = TextureBindingLayout::new()
            .with_view_dimension(TextureViewDimension::N2d)
            .with_sample_type(TextureSampleType::Float);

        let scene_sampler_entry =
            SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering);

        let world_position_texture_entry = TextureBindingLayout::new()
            .with_view_dimension(TextureViewDimension::N2d)
            // Better compatiblity with Rgba32float, which is used for world position textures
            // this pairs with the idea that we aren't passing a sampler for the world position texture
            // but rather using textureLoad() to read the texture directly
            .with_sample_type(TextureSampleType::UnfilterableFloat);


        let key = self
            .bind_groups
            .material_textures
            .insert_bind_group_layout(
                &self.gpu,
                vec![
                    MaterialTextureBindingLayoutEntry::Texture(scene_texture_entry),
                    MaterialTextureBindingLayoutEntry::Sampler(scene_sampler_entry),
                    MaterialTextureBindingLayoutEntry::Texture(world_position_texture_entry.clone()),
                    MaterialTextureBindingLayoutEntry::Texture(world_position_texture_entry),
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
        render_textures: &RenderTextureViews,
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
                &[
                    MaterialTextureBindingEntry::Texture(render_textures.scene.clone()),
                    MaterialTextureBindingEntry::Sampler(scene_texture_sampler),
                    MaterialTextureBindingEntry::Texture(render_textures.world_positions[0].clone()),
                    MaterialTextureBindingEntry::Texture(render_textures.world_positions[1].clone()),
                ],
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
