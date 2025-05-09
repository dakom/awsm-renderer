use awsm_renderer_core::sampler::{AddressMode, FilterMode, MipmapFilterMode, SamplerDescriptor};

use crate::{
    gltf::error::{AwsmGltfError, Result},
    materials::{pbr::PbrMaterialDeps, MaterialDeps, MaterialTextureDep},
    textures::{SamplerKey, TextureKey},
    AwsmRenderer,
};

use super::GltfPopulateContext;

pub fn gltf_material_deps(
    renderer: &mut AwsmRenderer,
    ctx: &GltfPopulateContext,
    material: gltf::Material,
) -> Result<MaterialDeps> {
    let mut deps = PbrMaterialDeps::default();

    if let Some(info) = material
        .pbr_metallic_roughness()
        .base_color_texture()
        .map(GltfTextureInfo::from)
    {
        deps.base_color = Some(info.create_dep(renderer, ctx)?);
    }

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
    pub fn create_dep(
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
                let texture_key = self.create_texture_key(renderer, ctx)?;
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

    fn create_texture_key(
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
            .create_texture(&renderer.gpu, None, false)
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

        let sampler = renderer.gpu.create_sampler(Some(&descriptor.into()));

        Ok(renderer.textures.add_sampler(sampler))
    }
}
