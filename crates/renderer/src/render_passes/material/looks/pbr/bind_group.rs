use std::borrow::Cow;

use awsm_renderer_core::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutResource, BindGroupResource,
        SamplerBindingLayout, SamplerBindingType, TextureBindingLayout,
    },
    renderer::AwsmRendererWebGpu,
    texture::{TextureSampleType, TextureViewDimension},
};

use crate::{
    bind_group_layout::{
        BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry, BindGroupLayoutKey,
    },
    bind_groups::AwsmBindGroupError,
    materials::{AwsmMaterialError, Result},
    textures::{SamplerKey, TextureKey, Textures},
};

// This stuff will likely go away, and instead we'll index into an array of textures
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
    pub fn into_bind_group(
        self,
        gpu: &AwsmRendererWebGpu,
        layout: &web_sys::GpuBindGroupLayout,
        textures: &Textures,
    ) -> Result<web_sys::GpuBindGroup> {
        let mut entries = Vec::new();

        // let mut push_texture = |dep: PbrMaterialTextureCacheKey| -> Result<()> {
        //     let texture = textures.get_texture(dep.texture_key)?;

        //     let texture_view = texture.create_view().map_err(|err| {
        //         AwsmMaterialError::CreateTextureView(format!("{:?}: {:?}", dep.texture_key, err))
        //     })?;

        //     let entry = BindGroupEntry::new(entries.len() as u32, BindGroupResource::TextureView(Cow::Owned(texture_view)));
        //     entries.push(entry);

        //     let sampler = textures.get_sampler(dep.sampler_key)?;

        //     let entry = BindGroupEntry::new(entries.len() as u32, BindGroupResource::Sampler(sampler));
        //     entries.push(entry);

        //     Ok(())
        // };

        // if let Some(tex) = self.base_color_tex {
        //     push_texture(tex)?;
        // }

        // if let Some(tex) = self.metallic_roughness_tex {
        //     push_texture(tex)?;
        // }
        // if let Some(tex) = self.normal_tex {
        //     push_texture(tex)?;
        // }
        // if let Some(tex) = self.occlusion_tex {
        //     push_texture(tex)?;
        // }
        // if let Some(tex) = self.emissive_tex {
        //     push_texture(tex)?;
        // }

        let descriptor = BindGroupDescriptor {
            label: Some("PbrMaterialBindGroup"),
            layout,
            entries,
        };

        Ok(gpu.create_bind_group(&descriptor.into()))
    }
}

impl From<&PbrMaterialBindGroupCacheKey> for BindGroupLayoutCacheKey {
    fn from(cache_key: &PbrMaterialBindGroupCacheKey) -> Self {
        let mut entries = Vec::new();

        let mut push_simple = || {
            let entry = TextureBindingLayout::new()
                .with_view_dimension(TextureViewDimension::N2d)
                .with_sample_type(TextureSampleType::Float);

            entries.push(BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(entry),
                visibility_compute: false,
                visibility_vertex: true,
                visibility_fragment: true,
            });

            let entry =
                SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering);

            entries.push(BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Sampler(entry),
                visibility_compute: false,
                visibility_vertex: true,
                visibility_fragment: true,
            });
        };

        if cache_key.base_color_tex.is_some() {
            push_simple();
        }

        if cache_key.metallic_roughness_tex.is_some() {
            push_simple();
        }
        if cache_key.normal_tex.is_some() {
            push_simple();
        }
        if cache_key.occlusion_tex.is_some() {
            push_simple();
        }
        if cache_key.emissive_tex.is_some() {
            push_simple();
        }

        BindGroupLayoutCacheKey { entries }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PbrMaterialTextureCacheKey {
    pub atlas_layer_index: usize,
    pub atlas_entry_index: usize,
}
