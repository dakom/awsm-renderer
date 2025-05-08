use awsm_renderer_core::{
    bind_groups::{SamplerBindingLayout, SamplerBindingType, TextureBindingLayout},
    renderer::AwsmRendererWebGpu,
    sampler::{AddressMode, FilterMode, MipmapFilterMode, SamplerDescriptor},
    texture::{TextureSampleType, TextureViewDimension},
};

use super::error::{AwsmGltfError, Result};
use crate::{
    bind_groups::material::{MaterialBindingEntry, MaterialBindingLayoutEntry},
    shaders::ShaderCacheKeyMaterial,
};

use super::populate::GltfPopulateContext;

// merely a key to hash ad-hoc material generation
#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GltfMaterialKey {
    // gltf index of the texture to use
    pub base_color: Option<GltfTextureInfo>,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct GltfTextureInfo {
    pub index: usize,
    pub tex_coord_index: usize,
}

impl GltfTextureInfo {
    pub fn create_texture_view(
        &self,
        ctx: &GltfPopulateContext,
    ) -> Result<web_sys::GpuTextureView> {
        let gltf_texture = ctx
            .data
            .doc
            .textures()
            .nth(self.index)
            .ok_or(AwsmGltfError::MissingTextureDocIndex(self.index))?;
        let texture_index = gltf_texture.source().index();
        let texture = ctx
            .data
            .textures
            .get(texture_index)
            .ok_or(AwsmGltfError::MissingTextureIndex(texture_index))?;

        let texture_view = texture
            .create_view()
            .map_err(|e| AwsmGltfError::CreateTextureView(format!("{e:?}")))?;

        Ok(texture_view)
    }

    pub fn create_sampler(
        &self,
        gpu: &AwsmRendererWebGpu,
        ctx: &GltfPopulateContext,
    ) -> Result<web_sys::GpuSampler> {
        let gltf_texture = ctx
            .data
            .doc
            .textures()
            .nth(self.index)
            .ok_or(AwsmGltfError::MissingTextureDocIndex(self.index))?;
        let gltf_sampler = gltf_texture.sampler();

        let mut descriptor = SamplerDescriptor::default();

        if let Some(mag_filter) = gltf_sampler.mag_filter() {
            match mag_filter {
                gltf::texture::MagFilter::Linear => {
                    descriptor.mag_filter = Some(FilterMode::Linear)
                }
                gltf::texture::MagFilter::Nearest => {
                    descriptor.mag_filter = Some(FilterMode::Nearest)
                }
            }
        }

        if let Some(min_filter) = gltf_sampler.min_filter() {
            match min_filter {
                gltf::texture::MinFilter::Linear => {
                    descriptor.min_filter = Some(FilterMode::Linear)
                }
                gltf::texture::MinFilter::Nearest => {
                    descriptor.min_filter = Some(FilterMode::Nearest)
                }
                gltf::texture::MinFilter::NearestMipmapNearest => {
                    descriptor.min_filter = Some(FilterMode::Nearest);
                    descriptor.mipmap_filter = Some(MipmapFilterMode::Nearest);
                }
                gltf::texture::MinFilter::LinearMipmapNearest => {
                    descriptor.min_filter = Some(FilterMode::Linear);
                    descriptor.mipmap_filter = Some(MipmapFilterMode::Nearest);
                }
                gltf::texture::MinFilter::NearestMipmapLinear => {
                    descriptor.min_filter = Some(FilterMode::Nearest);
                    descriptor.mipmap_filter = Some(MipmapFilterMode::Linear);
                }
                gltf::texture::MinFilter::LinearMipmapLinear => {
                    descriptor.min_filter = Some(FilterMode::Linear);
                    descriptor.mipmap_filter = Some(MipmapFilterMode::Linear);
                }
            }
        }

        match gltf_sampler.wrap_s() {
            gltf::texture::WrappingMode::ClampToEdge => {
                descriptor.address_mode_u = Some(AddressMode::ClampToEdge)
            }
            gltf::texture::WrappingMode::MirroredRepeat => {
                descriptor.address_mode_u = Some(AddressMode::MirrorRepeat)
            }
            gltf::texture::WrappingMode::Repeat => {
                descriptor.address_mode_u = Some(AddressMode::Repeat)
            }
        }

        match gltf_sampler.wrap_t() {
            gltf::texture::WrappingMode::ClampToEdge => {
                descriptor.address_mode_v = Some(AddressMode::ClampToEdge)
            }
            gltf::texture::WrappingMode::MirroredRepeat => {
                descriptor.address_mode_v = Some(AddressMode::MirrorRepeat)
            }
            gltf::texture::WrappingMode::Repeat => {
                descriptor.address_mode_v = Some(AddressMode::Repeat)
            }
        }

        let sampler = gpu.create_sampler(Some(&descriptor.into()));

        Ok(sampler)
    }
}

// similar, but just for the layout (re-used for different textures but same material layout)
#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GltfMaterialLayoutKey {
    pub base_color: bool,
}

impl GltfMaterialKey {
    #[allow(private_interfaces)]
    pub fn new(material: gltf::Material) -> Self {
        let mut key = Self::default();

        if let Some(info) = material.pbr_metallic_roughness().base_color_texture() {
            let texture_index = info.texture().index();
            let tex_coord_index = info.tex_coord();

            key.base_color = Some(GltfTextureInfo {
                index: texture_index,
                tex_coord_index: tex_coord_index as usize,
            });
        }

        key
    }

    pub fn layout_key(&self) -> GltfMaterialLayoutKey {
        GltfMaterialLayoutKey {
            base_color: self.base_color.is_some(),
        }
    }

    pub fn shader_cache_key(&self) -> ShaderCacheKeyMaterial {
        let mut key = ShaderCacheKeyMaterial::default();
        if let Some(info) = self.base_color {
            key.base_color_tex_coord_index = Some(info.tex_coord_index as u32);
        }
        key
    }

    pub fn entries(
        &self,
        gpu: &AwsmRendererWebGpu,
        ctx: &GltfPopulateContext,
    ) -> Result<Vec<MaterialBindingEntry>> {
        let mut entries = Vec::new();

        // make sure the order matches shader.rs!
        if let Some(texture_info) = self.base_color {
            let texture_view = texture_info.create_texture_view(ctx)?;
            let entry = MaterialBindingEntry::Texture(texture_view);
            entries.push(entry);

            let sampler = texture_info.create_sampler(gpu, ctx)?;
            let entry = MaterialBindingEntry::Sampler(sampler);
            entries.push(entry);
        }

        Ok(entries)
    }
}

impl GltfMaterialLayoutKey {
    pub fn layout_entries(&self) -> Vec<MaterialBindingLayoutEntry> {
        let mut entries = Vec::new();

        if self.base_color {
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
}
