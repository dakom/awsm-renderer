use gltf::material::AlphaMode as GltfAlphaMode;

use crate::prelude::*;
use crate::renderer::material::{Material, PbrMaterial, TextureInfo};
use super::populate::GltfPopulateContext;
use super::loader::GltfResource;

impl AwsmRenderer {
    pub(super) fn gltf_set_material_texture_uniforms(&mut self, world: &World, res: &GltfResource, ctx: &mut GltfPopulateContext, material: &mut PbrMaterial, gltf_material: &gltf::Material) -> Result<()> {
        let gltf_metallic_roughness = gltf_material.pbr_metallic_roughness();

        material.alpha_mode = Some(match gltf_material.alpha_mode() {
            GltfAlphaMode::Opaque => AlphaMode::Opaque,
            GltfAlphaMode::Blend => AlphaMode::Blend,
            GltfAlphaMode::Mask => AlphaMode::Mask { cutoff: gltf_material.alpha_cutoff().unwrap_or(0.5) } // 0.5 is default defined in spec
        });

        material.base_color_factor = gltf_metallic_roughness.base_color_factor().into();
        material.metallic_factor = gltf_metallic_roughness.metallic_factor().into();
        material.roughness_factor = gltf_metallic_roughness.roughness_factor().into();
        material.emissive_factor = gltf_material.emissive_factor().into();

        if let Some(info) = gltf_metallic_roughness.base_color_texture() {
            material.base_color_texture = Some(TextureInfo {
                    id: self.gltf_get_texture(res, ctx, &info.texture())?,
                    uv_index: info.tex_coord()
            });
        }

        if let Some(info) = gltf_metallic_roughness.metallic_roughness_texture() {
            material.metallic_roughness_texture = Some(TextureInfo {
                    id: self.gltf_get_texture(res, ctx, &info.texture())?,
                    uv_index: info.tex_coord()
            });
        }

        if let Some(info) = gltf_material.normal_texture() {
            material.normal_texture = Some(TextureInfo {
                    id: self.gltf_get_texture(res, ctx, &info.texture())?,
                    uv_index: info.tex_coord()
            });
        }

        if let Some(info) = gltf_material.emissive_texture() {
            material.emissive_texture = Some(TextureInfo {
                    id: self.gltf_get_texture(res, ctx, &info.texture())?,
                    uv_index: info.tex_coord()
            });
        }

        if gltf_material.double_sided() {
            material.double_sided = true;
        }


        Ok(())
    }
}
