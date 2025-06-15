use awsm_renderer_core::sampler::{AddressMode, FilterMode, MipmapFilterMode, SamplerDescriptor};

use crate::{
    gltf::error::{AwsmGltfError, Result},
    materials::{pbr::PbrMaterialDeps, MaterialAlphaMode, MaterialDeps, MaterialTextureDep},
    textures::{SamplerKey, TextureKey},
    AwsmRenderer,
};

use super::GltfPopulateContext;

pub async fn gltf_material_deps(
    renderer: &mut AwsmRenderer,
    ctx: &GltfPopulateContext,
    material: gltf::Material<'_>,
) -> Result<MaterialDeps> {
    let mut deps = PbrMaterialDeps::default();

    let pbr = material.pbr_metallic_roughness();

    deps.base_color_factor = pbr.base_color_factor();

    if let Some(tex) = pbr.base_color_texture().map(GltfTextureInfo::from) {
        deps.base_color_tex = Some(tex.create_dep(renderer, ctx).await?);
    }

    deps.metallic_factor = pbr.metallic_factor();
    deps.roughness_factor = pbr.roughness_factor();

    if let Some(tex) = pbr.metallic_roughness_texture().map(GltfTextureInfo::from) {
        deps.metallic_roughness_tex = Some(tex.create_dep(renderer, ctx).await?);
    }

    if let Some(normal_tex) = material.normal_texture() {
        let tex = GltfTextureInfo {
            index: normal_tex.texture().index(),
            tex_coord_index: normal_tex.tex_coord() as usize,
        };
        deps.normal_tex = Some(tex.create_dep(renderer, ctx).await?);
        deps.normal_scale = normal_tex.scale();
    }

    if let Some(occlusion_tex) = material.occlusion_texture() {
        let tex = GltfTextureInfo {
            index: occlusion_tex.texture().index(),
            tex_coord_index: occlusion_tex.tex_coord() as usize,
        };
        deps.occlusion_tex = Some(tex.create_dep(renderer, ctx).await?);
        deps.occlusion_strength = occlusion_tex.strength();
    }

    if let Some(emissive_tex) = material.emissive_texture() {
        let tex = GltfTextureInfo {
            index: emissive_tex.texture().index(),
            tex_coord_index: emissive_tex.tex_coord() as usize,
        };
        deps.emissive_tex = Some(tex.create_dep(renderer, ctx).await?);
    }

    deps.emissive_factor = material.emissive_factor();
    deps.alpha_mode = match material.alpha_mode() {
        gltf::material::AlphaMode::Opaque => MaterialAlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => MaterialAlphaMode::Mask {
            cutoff: material.alpha_cutoff().unwrap_or(0.5),
        },
        gltf::material::AlphaMode::Blend => MaterialAlphaMode::Blend,
    };
    deps.double_sided = material.double_sided();

    Ok(MaterialDeps::Pbr(deps))
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

impl GltfTextureInfo {
    pub async fn create_dep(
        &self,
        renderer: &mut AwsmRenderer,
        ctx: &GltfPopulateContext,
    ) -> Result<MaterialTextureDep> {
        let dep = {
            let lock = ctx.textures.lock().unwrap();
            lock.get(&self.index).cloned()
        };

        let (texture_key, sampler_key) = match dep {
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

        Ok(MaterialTextureDep {
            texture_key,
            sampler_key,
            uv_index: self.tex_coord_index,
        })
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
