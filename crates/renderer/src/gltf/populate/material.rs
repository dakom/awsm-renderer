use awsm_renderer_core::{
    sampler::{AddressMode, FilterMode, MipmapFilterMode, SamplerDescriptor},
    texture::{mipmap::MipmapTextureKind, texture_pool::TextureColorInfo, TextureFormat},
};
use ordered_float::OrderedFloat;

use crate::{
    gltf::{
        buffers::MeshBufferInfoWithOffset,
        error::{AwsmGltfError, Result},
        populate::GltfTextureKey,
    },
    materials::{
        pbr::{PbrMaterial, VertexColorInfo},
        MaterialAlphaMode,
    },
    mesh::{MeshBufferCustomVertexAttributeInfo, MeshBufferInfo, MeshBufferVertexAttributeInfo},
    textures::{SamplerCacheKey, SamplerKey, TextureKey, TextureTransform, TextureTransformKey},
    AwsmRenderer,
};

use super::GltfPopulateContext;

pub struct GltfMaterialInfo {
    pub material: PbrMaterial,
}

impl GltfMaterialInfo {
    pub async fn new(
        renderer: &mut AwsmRenderer,
        ctx: &GltfPopulateContext,
        primitive_buffer_info: &MeshBufferInfoWithOffset,
        gltf_material: gltf::Material<'_>,
    ) -> Result<Self> {
        let (alpha_mode, premultiplied_alpha) = match gltf_material.alpha_mode() {
            gltf::material::AlphaMode::Opaque => (MaterialAlphaMode::Opaque, None),
            gltf::material::AlphaMode::Mask => (
                MaterialAlphaMode::Mask {
                    cutoff: gltf_material.alpha_cutoff().unwrap_or(0.5),
                },
                Some(false),
            ),
            gltf::material::AlphaMode::Blend => (MaterialAlphaMode::Blend, Some(false)),
        };
        let mut material = PbrMaterial::new(alpha_mode, gltf_material.double_sided());

        let pbr = gltf_material.pbr_metallic_roughness();

        if let Some(tex) = pbr.base_color_texture().map(GltfTextureInfo::from) {
            let GLtfMaterialCacheKey {
                uv_index,
                texture_key,
                sampler_key,
                texture_transform_key,
            } = tex
                .create_material_cache_key(
                    renderer,
                    ctx,
                    TextureColorInfo {
                        mipmap_kind: MipmapTextureKind::Albedo,
                        srgb_to_linear: true,
                        premultiplied_alpha,
                    },
                )
                .await?;

            material.base_color_tex = Some(texture_key);
            material.base_color_sampler = Some(sampler_key);
            material.base_color_uv_index = Some(uv_index as u32);
            material.base_color_texture_transform = texture_transform_key;
        }

        if let Some(tex) = pbr.metallic_roughness_texture().map(GltfTextureInfo::from) {
            let GLtfMaterialCacheKey {
                uv_index,
                texture_key,
                sampler_key,
                texture_transform_key,
            } = tex
                .create_material_cache_key(
                    renderer,
                    ctx,
                    TextureColorInfo {
                        mipmap_kind: MipmapTextureKind::MetallicRoughness,
                        srgb_to_linear: false,
                        premultiplied_alpha,
                    },
                )
                .await?;
            material.metallic_roughness_tex = Some(texture_key);
            material.metallic_roughness_sampler = Some(sampler_key);
            material.metallic_roughness_uv_index = Some(uv_index as u32);
            material.metallic_roughness_texture_transform = texture_transform_key;
        }

        if let Some(tex) = gltf_material.normal_texture().map(GltfTextureInfo::from) {
            let GLtfMaterialCacheKey {
                uv_index,
                texture_key,
                sampler_key,
                texture_transform_key,
            } = tex
                .create_material_cache_key(
                    renderer,
                    ctx,
                    TextureColorInfo {
                        mipmap_kind: MipmapTextureKind::Normal,
                        srgb_to_linear: false,
                        premultiplied_alpha,
                    },
                )
                .await?;

            material.normal_tex = Some(texture_key);
            material.normal_sampler = Some(sampler_key);
            material.normal_uv_index = Some(uv_index as u32);
            material.normal_texture_transform = texture_transform_key;
        }

        if let Some(tex) = gltf_material.occlusion_texture().map(GltfTextureInfo::from) {
            let GLtfMaterialCacheKey {
                uv_index,
                texture_key,
                sampler_key,
                texture_transform_key,
            } = tex
                .create_material_cache_key(
                    renderer,
                    ctx,
                    TextureColorInfo {
                        mipmap_kind: MipmapTextureKind::Occlusion,
                        srgb_to_linear: false,
                        premultiplied_alpha,
                    },
                )
                .await?;

            material.occlusion_tex = Some(texture_key);
            material.occlusion_sampler = Some(sampler_key);
            material.occlusion_uv_index = Some(uv_index as u32);
            material.occlusion_texture_transform = texture_transform_key;
        }

        if let Some(tex) = gltf_material.emissive_texture().map(GltfTextureInfo::from) {
            let GLtfMaterialCacheKey {
                uv_index,
                texture_key,
                sampler_key,
                texture_transform_key,
            } = tex
                .create_material_cache_key(
                    renderer,
                    ctx,
                    TextureColorInfo {
                        mipmap_kind: MipmapTextureKind::Emissive,
                        srgb_to_linear: true,
                        premultiplied_alpha,
                    },
                )
                .await?;

            material.emissive_tex = Some(texture_key);
            material.emissive_sampler = Some(sampler_key);
            material.emissive_uv_index = Some(uv_index as u32);
            material.emissive_texture_transform = texture_transform_key;
        }

        if let Some(normal_tex) = gltf_material.normal_texture() {
            material.normal_scale = normal_tex.scale();
        }

        if let Some(occlusion_tex) = gltf_material.occlusion_texture() {
            material.occlusion_strength = occlusion_tex.strength();
        }
        material.emissive_factor = gltf_material.emissive_factor();
        material.emissive_strength = gltf_material.emissive_strength().unwrap_or(1.0);

        let pbr = gltf_material.pbr_metallic_roughness();
        material.base_color_factor = pbr.base_color_factor();
        material.metallic_factor = pbr.metallic_factor();
        material.roughness_factor = pbr.roughness_factor();

        material.vertex_color_info = primitive_buffer_info
            .triangles
            .vertex_attributes
            .iter()
            .find_map(|attr| {
                if let &MeshBufferVertexAttributeInfo::Custom(
                    MeshBufferCustomVertexAttributeInfo::Colors { index, .. },
                ) = attr
                {
                    // for right now just always use the first one we find
                    Some(VertexColorInfo { set_index: index })
                } else {
                    None
                }
            });

        Ok(Self { material })
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct GltfTextureInfo {
    pub index: usize,
    pub tex_coord_index: usize,
    pub texture_transform: Option<GltfTextureTransform>,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct GltfTextureTransform {
    // The offset of the UV coordinate origin as a factor of the texture dimensions.
    pub offset: [OrderedFloat<f32>; 2],

    /// Rotate the UVs by this many radians counter-clockwise around the origin.
    /// This is equivalent to a similar rotation of the image clockwise.
    pub rotation: OrderedFloat<f32>,

    /// The scale factor applied to the components of the UV coordinates.
    pub scale: [OrderedFloat<f32>; 2],
}

impl<'a> From<gltf::texture::Info<'a>> for GltfTextureInfo {
    fn from(info: gltf::texture::Info<'a>) -> Self {
        Self {
            index: info.texture().index(),
            tex_coord_index: match info.texture_transform().and_then(|x| x.tex_coord()) {
                Some(tex_coord_index) => tex_coord_index,
                None => info.tex_coord(),
            } as usize,
            texture_transform: info.texture_transform().map(GltfTextureTransform::from),
        }
    }
}

impl<'a> From<gltf::material::NormalTexture<'a>> for GltfTextureInfo {
    fn from(info: gltf::material::NormalTexture<'a>) -> Self {
        // Extract KHR_texture_transform from extensions if present
        let texture_transform = info.extensions().and_then(|ext| {
            ext.get("KHR_texture_transform").and_then(|transform_json| {
                // Parse the extension manually
                let offset = transform_json
                    .get("offset")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        [
                            arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        ]
                    })
                    .unwrap_or([0.0, 0.0]);

                let rotation = transform_json
                    .get("rotation")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;

                let scale = transform_json
                    .get("scale")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        [
                            arr.get(0).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                            arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                        ]
                    })
                    .unwrap_or([1.0, 1.0]);

                Some(GltfTextureTransform {
                    offset: [
                        ordered_float::OrderedFloat(offset[0]),
                        ordered_float::OrderedFloat(offset[1]),
                    ],
                    rotation: ordered_float::OrderedFloat(rotation),
                    scale: [
                        ordered_float::OrderedFloat(scale[0]),
                        ordered_float::OrderedFloat(scale[1]),
                    ],
                })
            })
        });

        let tex_coord_override = info
            .extensions()
            .and_then(|ext| ext.get("KHR_texture_transform"))
            .and_then(|t| t.get("texCoord"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        Self {
            index: info.texture().index(),
            tex_coord_index: tex_coord_override.unwrap_or(info.tex_coord() as usize),
            texture_transform,
        }
    }
}

impl<'a> From<gltf::material::OcclusionTexture<'a>> for GltfTextureInfo {
    fn from(info: gltf::material::OcclusionTexture<'a>) -> Self {
        // Extract KHR_texture_transform from extensions if present
        let texture_transform = info.extensions().and_then(|ext| {
            ext.get("KHR_texture_transform").and_then(|transform_json| {
                // Parse the extension manually
                let offset = transform_json
                    .get("offset")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        [
                            arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        ]
                    })
                    .unwrap_or([0.0, 0.0]);

                let rotation = transform_json
                    .get("rotation")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;

                let scale = transform_json
                    .get("scale")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        [
                            arr.get(0).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                            arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                        ]
                    })
                    .unwrap_or([1.0, 1.0]);

                Some(GltfTextureTransform {
                    offset: [
                        ordered_float::OrderedFloat(offset[0]),
                        ordered_float::OrderedFloat(offset[1]),
                    ],
                    rotation: ordered_float::OrderedFloat(rotation),
                    scale: [
                        ordered_float::OrderedFloat(scale[0]),
                        ordered_float::OrderedFloat(scale[1]),
                    ],
                })
            })
        });

        let tex_coord_override = info
            .extensions()
            .and_then(|ext| ext.get("KHR_texture_transform"))
            .and_then(|t| t.get("texCoord"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        Self {
            index: info.texture().index(),
            tex_coord_index: tex_coord_override.unwrap_or(info.tex_coord() as usize),
            texture_transform,
        }
    }
}

impl<'a> From<gltf::texture::TextureTransform<'a>> for GltfTextureTransform {
    fn from(transform: gltf::texture::TextureTransform<'a>) -> Self {
        Self {
            offset: [
                OrderedFloat(transform.offset()[0]),
                OrderedFloat(transform.offset()[1]),
            ],
            rotation: OrderedFloat(transform.rotation()),
            scale: [
                OrderedFloat(transform.scale()[0]),
                OrderedFloat(transform.scale()[1]),
            ],
        }
    }
}

type UvIndex = usize;

pub struct GLtfMaterialCacheKey {
    pub uv_index: usize,
    pub texture_key: TextureKey,
    pub sampler_key: SamplerKey,
    pub texture_transform_key: Option<TextureTransformKey>,
}
impl GltfTextureInfo {
    pub async fn create_material_cache_key(
        &self,
        renderer: &mut AwsmRenderer,
        ctx: &GltfPopulateContext,
        color: TextureColorInfo,
    ) -> Result<GLtfMaterialCacheKey> {
        let lookup_key = GltfTextureKey {
            index: self.index,
            color,
        };

        let sampler_key = self.create_sampler_key(renderer, ctx)?;

        let texture_key = {
            let textures = ctx.textures.lock().unwrap();
            textures.get(&lookup_key).cloned()
        };

        let texture_key = match texture_key {
            Some(texture_key) => texture_key,
            None => {
                let gltf_texture = ctx
                    .data
                    .doc
                    .textures()
                    .nth(self.index)
                    .ok_or(AwsmGltfError::MissingTextureDocIndex(self.index))?;
                let texture_index = gltf_texture.source().index();
                let image_data = ctx
                    .data
                    .images
                    .get(texture_index)
                    .ok_or(AwsmGltfError::MissingTextureIndex(texture_index))?;

                let texture_key = renderer.textures.add_image(
                    image_data.clone(),
                    image_data.format(),
                    sampler_key,
                    color,
                )?;

                ctx.textures.lock().unwrap().insert(lookup_key, texture_key);

                texture_key
            }
        };

        let texture_transform_key = match self.texture_transform {
            None => None,
            Some(texture_transform) => Some(renderer.textures.insert_texture_transform(
                &TextureTransform {
                    offset: [*texture_transform.offset[0], *texture_transform.offset[1]],
                    origin: [0.0, 0.0],
                    rotation: *texture_transform.rotation,
                    scale: [*texture_transform.scale[0], *texture_transform.scale[1]],
                },
            )),
        };

        Ok(GLtfMaterialCacheKey {
            uv_index: self.tex_coord_index,
            texture_key,
            sampler_key,
            texture_transform_key,
        })
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

        let mut sampler_cache_key = SamplerCacheKey {
            // This looks better with our mipmap generation...
            // if it's overridden by the glTF sampler, fine.
            // but otherwise, let's just do what looks best.
            min_filter: Some(FilterMode::Linear),
            mag_filter: Some(FilterMode::Linear),
            mipmap_filter: Some(MipmapFilterMode::Linear),
            // Enable anisotropic filtering for thin lines at oblique angles
            // Without this, textures become severely aliased when viewed at angles
            max_anisotropy: Some(16),
            ..Default::default()
        };
        // glTF allows omitting the wrap mode; the spec states the default is repeat. Record that
        // here so downstream shader logic can faithfully emulate it if the sampler isn't cached yet.
        sampler_cache_key.address_mode_u = Some(AddressMode::Repeat);
        sampler_cache_key.address_mode_v = Some(AddressMode::Repeat);
        sampler_cache_key.address_mode_w = Some(AddressMode::Repeat);

        if let Some(mag_filter) = gltf_sampler.mag_filter() {
            match mag_filter {
                gltf::texture::MagFilter::Linear => {
                    sampler_cache_key.mag_filter = Some(FilterMode::Linear)
                }
                gltf::texture::MagFilter::Nearest => {
                    sampler_cache_key.mag_filter = Some(FilterMode::Nearest)
                }
            }
        }

        if let Some(min_filter) = gltf_sampler.min_filter() {
            match min_filter {
                gltf::texture::MinFilter::Linear => {
                    sampler_cache_key.min_filter = Some(FilterMode::Linear)
                }
                gltf::texture::MinFilter::Nearest => {
                    sampler_cache_key.min_filter = Some(FilterMode::Nearest)
                }
                gltf::texture::MinFilter::NearestMipmapNearest => {
                    sampler_cache_key.min_filter = Some(FilterMode::Nearest);
                    sampler_cache_key.mipmap_filter = Some(MipmapFilterMode::Nearest);
                }
                gltf::texture::MinFilter::LinearMipmapNearest => {
                    sampler_cache_key.min_filter = Some(FilterMode::Linear);
                    sampler_cache_key.mipmap_filter = Some(MipmapFilterMode::Nearest);
                }
                gltf::texture::MinFilter::NearestMipmapLinear => {
                    sampler_cache_key.min_filter = Some(FilterMode::Nearest);
                    sampler_cache_key.mipmap_filter = Some(MipmapFilterMode::Linear);
                }
                gltf::texture::MinFilter::LinearMipmapLinear => {
                    sampler_cache_key.min_filter = Some(FilterMode::Linear);
                    sampler_cache_key.mipmap_filter = Some(MipmapFilterMode::Linear);
                }
            }
        }

        match gltf_sampler.wrap_s() {
            gltf::texture::WrappingMode::ClampToEdge => {
                sampler_cache_key.address_mode_u = Some(AddressMode::ClampToEdge)
            }
            gltf::texture::WrappingMode::MirroredRepeat => {
                sampler_cache_key.address_mode_u = Some(AddressMode::MirrorRepeat)
            }
            gltf::texture::WrappingMode::Repeat => {
                sampler_cache_key.address_mode_u = Some(AddressMode::Repeat)
            }
        }

        match gltf_sampler.wrap_t() {
            gltf::texture::WrappingMode::ClampToEdge => {
                sampler_cache_key.address_mode_v = Some(AddressMode::ClampToEdge)
            }
            gltf::texture::WrappingMode::MirroredRepeat => {
                sampler_cache_key.address_mode_v = Some(AddressMode::MirrorRepeat)
            }
            gltf::texture::WrappingMode::Repeat => {
                sampler_cache_key.address_mode_v = Some(AddressMode::Repeat)
            }
        }

        if !sampler_cache_key.allowed_ansiotropy() {
            //tracing::warn!("Disabling max ansiotropy!");
            sampler_cache_key.max_anisotropy = None;
        }

        Ok(renderer
            .textures
            .get_sampler_key(&renderer.gpu, sampler_cache_key)?)
    }
}
