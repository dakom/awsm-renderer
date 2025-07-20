use awsm_renderer_core::sampler::{AddressMode, FilterMode, MipmapFilterMode, SamplerDescriptor};

use crate::{
    gltf::error::{AwsmGltfError, Result},
    materials::{
        pbr::{
            PbrMaterial, PbrMaterialBindGroupCacheKey, PbrMaterialBindGroupLayoutCacheKey,
            PbrMaterialTextureCacheKey,
        },
        MaterialAlphaMode,
    },
    shaders::pbr::PbrShaderCacheKeyMaterial,
    textures::{SamplerKey, TextureKey},
    AwsmRenderer,
};

use super::GltfPopulateContext;

pub struct GltfMaterialInfo {
    pub bind_group_cache_key: PbrMaterialBindGroupCacheKey,
    pub bind_group_layout_cache_key: PbrMaterialBindGroupLayoutCacheKey,
    pub shader_cache_key: PbrShaderCacheKeyMaterial,
    pub material: PbrMaterial,
}

impl GltfMaterialInfo {
    pub async fn new(
        renderer: &mut AwsmRenderer,
        ctx: &GltfPopulateContext,
        gltf_material: gltf::Material<'_>,
    ) -> Result<Self> {
        let mut bind_group_cache_key = PbrMaterialBindGroupCacheKey::default();
        let mut shader_cache_key = PbrShaderCacheKeyMaterial::default();

        let pbr = gltf_material.pbr_metallic_roughness();

        if let Some(tex) = pbr.base_color_texture().map(GltfTextureInfo::from) {
            let (uv_index, texture_cache_key) =
                tex.create_material_cache_key(renderer, ctx).await?;
            bind_group_cache_key.base_color_tex = Some(texture_cache_key);
            shader_cache_key.base_color_uv_index = Some(uv_index as u32);
        }

        if let Some(tex) = pbr.metallic_roughness_texture().map(GltfTextureInfo::from) {
            let (uv_index, texture_cache_key) =
                tex.create_material_cache_key(renderer, ctx).await?;
            bind_group_cache_key.metallic_roughness_tex = Some(texture_cache_key);
            shader_cache_key.metallic_roughness_uv_index = Some(uv_index as u32);
        }

        if let Some(normal_tex) = gltf_material.normal_texture() {
            let tex = GltfTextureInfo {
                index: normal_tex.texture().index(),
                tex_coord_index: normal_tex.tex_coord() as usize,
            };
            let (uv_index, tex) = tex.create_material_cache_key(renderer, ctx).await?;
            bind_group_cache_key.normal_tex = Some(tex);
            shader_cache_key.normal_uv_index = Some(uv_index as u32);
        }

        if let Some(occlusion_tex) = gltf_material.occlusion_texture() {
            let tex = GltfTextureInfo {
                index: occlusion_tex.texture().index(),
                tex_coord_index: occlusion_tex.tex_coord() as usize,
            };
            let (uv_index, tex) = tex.create_material_cache_key(renderer, ctx).await?;
            bind_group_cache_key.occlusion_tex = Some(tex);
            shader_cache_key.occlusion_uv_index = Some(uv_index as u32);
        }

        if let Some(emissive_tex) = gltf_material.emissive_texture() {
            let tex = GltfTextureInfo {
                index: emissive_tex.texture().index(),
                tex_coord_index: emissive_tex.tex_coord() as usize,
            };
            let (uv_index, tex) = tex.create_material_cache_key(renderer, ctx).await?;
            bind_group_cache_key.emissive_tex = Some(tex);
            shader_cache_key.emissive_uv_index = Some(uv_index as u32);
        }

        shader_cache_key.has_alpha_mask =
            matches!(gltf_material.alpha_mode(), gltf::material::AlphaMode::Mask);

        let alpha_mode = match gltf_material.alpha_mode() {
            gltf::material::AlphaMode::Opaque => MaterialAlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => MaterialAlphaMode::Mask {
                cutoff: gltf_material.alpha_cutoff().unwrap_or(0.5),
            },
            gltf::material::AlphaMode::Blend => MaterialAlphaMode::Blend,
        };
        let mut material = PbrMaterial::new(alpha_mode, gltf_material.double_sided());
        let bind_group_layout_cache_key = (&bind_group_cache_key).into();

        if let Some(normal_tex) = gltf_material.normal_texture() {
            material.normal_scale = normal_tex.scale();
        }

        if let Some(occlusion_tex) = gltf_material.occlusion_texture() {
            material.occlusion_strength = occlusion_tex.strength();
        }
        material.emissive_factor = gltf_material.emissive_factor();

        let pbr = gltf_material.pbr_metallic_roughness();
        material.base_color_factor = pbr.base_color_factor();
        material.metallic_factor = pbr.metallic_factor();
        material.roughness_factor = pbr.roughness_factor();

        Ok(Self {
            bind_group_cache_key,
            bind_group_layout_cache_key,
            shader_cache_key,
            material,
        })
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct GltfTextureInfo {
    pub index: usize,
    pub tex_coord_index: usize,
}

impl<'a> From<gltf::texture::Info<'a>> for GltfTextureInfo {
    fn from(info: gltf::texture::Info<'a>) -> Self {
        Self {
            index: info.texture().index(),
            tex_coord_index: info.tex_coord() as usize,
        }
    }
}

type UvIndex = usize;
impl GltfTextureInfo {
    pub async fn create_material_cache_key(
        &self,
        renderer: &mut AwsmRenderer,
        ctx: &GltfPopulateContext,
    ) -> Result<(UvIndex, PbrMaterialTextureCacheKey)> {
        let texture_cache_key = {
            let lock = ctx.textures.lock().unwrap();
            lock.get(&self.index).cloned()
        };

        let (texture_key, sampler_key) = match texture_cache_key {
            Some((texture_key, sampler_key)) => (texture_key, sampler_key),
            None => {
                let texture_key = self.create_texture_key(renderer, ctx).await?;
                let sampler_key = self.create_sampler_key(renderer, ctx)?;
                ctx.textures
                    .lock()
                    .unwrap()
                    .insert(self.index, (texture_key, sampler_key));
                (texture_key, sampler_key)
            }
        };

        Ok((
            self.tex_coord_index,
            PbrMaterialTextureCacheKey {
                texture_key,
                sampler_key,
            },
        ))
    }

    async fn create_texture_key(
        &self,
        renderer: &mut AwsmRenderer,
        ctx: &GltfPopulateContext,
    ) -> Result<TextureKey> {
        let gltf_texture = ctx
            .data
            .doc
            .textures()
            .nth(self.index)
            .ok_or(AwsmGltfError::MissingTextureDocIndex(self.index))?;
        let texture_index = gltf_texture.source().index();
        let image = ctx
            .data
            .images
            .get(texture_index)
            .ok_or(AwsmGltfError::MissingTextureIndex(texture_index))?;

        let texture = image
            .create_texture(&renderer.gpu, None, ctx.generate_mipmaps)
            .await
            .map_err(AwsmGltfError::CreateTexture)?;

        Ok(renderer.textures.add_texture(texture))
    }

    fn create_sampler_key(
        &self,
        renderer: &mut AwsmRenderer,
        ctx: &GltfPopulateContext,
    ) -> Result<SamplerKey> {
        let gltf_texture = ctx
            .data
            .doc
            .textures()
            .nth(self.index)
            .ok_or(AwsmGltfError::MissingTextureDocIndex(self.index))?;
        let gltf_sampler = gltf_texture.sampler();

        let mut descriptor = SamplerDescriptor {
            // This looks better with our mipmap generation...
            // if it's overridden by the glTF sampler, fine.
            // but otherwise, let's just do what looks best.
            min_filter: Some(FilterMode::Linear),
            mag_filter: Some(FilterMode::Linear),
            mipmap_filter: Some(MipmapFilterMode::Linear),
            ..SamplerDescriptor::default()
        };

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

        let sampler = renderer.gpu.create_sampler(Some(&descriptor.into()));

        Ok(renderer.textures.add_sampler(sampler))
    }
}
