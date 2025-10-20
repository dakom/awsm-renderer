use std::borrow::Cow;
use std::collections::BTreeSet;

use awsm_renderer_core::bind_groups::{
    self, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutResource, BindGroupResource,
    BufferBindingLayout, BufferBindingType, SamplerBindingLayout, SamplerBindingType,
    StorageTextureAccess, StorageTextureBindingLayout, TextureBindingLayout,
};
use awsm_renderer_core::buffers::BufferBinding;
use awsm_renderer_core::error::AwsmCoreError;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use awsm_renderer_core::texture::{self, TextureSampleType, TextureViewDimension};

use crate::bind_group_layout::{BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry};
use crate::bind_groups::{AwsmBindGroupError, BindGroupRecreateContext};
use crate::camera::AwsmCameraError;
use crate::error::Result;
use crate::materials::pbr::PbrMaterial;
use crate::materials::MaterialBufferKind;
use crate::mesh::meta::material_opaque_meta::MATERIAL_MESH_META_BYTE_ALIGNMENT;
use crate::textures::SamplerKey;
use crate::{bind_group_layout::BindGroupLayoutKey, render_passes::RenderPassInitContext};

pub const MATERIAL_OPAQUE_CORE_TEXTURES_START_GROUP: u32 = 1;
pub const MATERIAL_OPAQUE_CORE_TEXTURES_START_BINDING: u32 = 0;

pub struct MaterialOpaqueBindGroups {
    pub main_bind_group_layout_key: BindGroupLayoutKey,
    pub texture_bind_group_layout_key: BindGroupLayoutKey,
    pub sampler_bind_group_layout_key: BindGroupLayoutKey,
    pub texture_atlas_len: u32,
    pub texture_sampler_keys: BTreeSet<SamplerKey>,
    // this is set via `recreate` mechanism
    _main_bind_group: Option<web_sys::GpuBindGroup>,
    _texture_bind_group: Option<web_sys::GpuBindGroup>,
    _sampler_bind_group: Option<web_sys::GpuBindGroup>,
}

impl MaterialOpaqueBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let main_entries = vec![
            // Mesh Meta (for this pass, different than geometry pass)
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Visibility data texture
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(
                    TextureBindingLayout::new()
                        .with_view_dimension(TextureViewDimension::N2d)
                        .with_sample_type(TextureSampleType::UnfilterableFloat),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Opaque color render texture (storage texture for compute write)
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::StorageTexture(
                    StorageTextureBindingLayout::new(ctx.render_texture_formats.opaque_color)
                        .with_view_dimension(TextureViewDimension::N2d)
                        .with_access(StorageTextureAccess::WriteOnly),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Material data buffer
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Attribute index buffer
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Attribute data buffer
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Transform buffer
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Normal matrices buffer
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Camera uniform gives us inverse matrices + frustum rays for depth reprojection.
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new().with_binding_type(BufferBindingType::Uniform),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Skybox texture
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(
                    TextureBindingLayout::new().with_view_dimension(TextureViewDimension::Cube),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Skybox sampler
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Sampler(
                    SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // IBL prefiltered env texture
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(
                    TextureBindingLayout::new().with_view_dimension(TextureViewDimension::Cube),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // IBL prefiltered env sampler
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Sampler(
                    SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // IBL irradiance env texture
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(
                    TextureBindingLayout::new().with_view_dimension(TextureViewDimension::Cube),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // IBL irradiance env sampler
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Sampler(
                    SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // IBL info buffer
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new().with_binding_type(BufferBindingType::Uniform),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Brdf lut texture
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(
                    TextureBindingLayout::new().with_view_dimension(TextureViewDimension::N2d),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Brdf lut sampler
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Sampler(
                    SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Depth texture from the visibility pass – sampled during the compute shading stage.
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(
                    TextureBindingLayout::new()
                        .with_view_dimension(TextureViewDimension::N2d)
                        .with_sample_type(TextureSampleType::Depth),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            // Visibility data buffer (positions, triangle-id, barycentric) for mipmap computation
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
        ];

        let main_bind_group_layout_key = ctx.bind_group_layouts.get_key(
            &ctx.gpu,
            BindGroupLayoutCacheKey {
                entries: main_entries,
            },
        )?;

        // textures
        let device_limits = ctx.gpu.device.limits();
        let texture_atlas_len = ctx.textures.mega_texture.bindings_len(&device_limits)?;

        let mut texture_entries = Vec::new();

        for i in 0..texture_atlas_len {
            texture_entries.push(BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(
                    TextureBindingLayout::new()
                        .with_view_dimension(TextureViewDimension::N2dArray)
                        .with_sample_type(TextureSampleType::Float),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            });
        }

        let texture_bind_group_layout_key = ctx.bind_group_layouts.get_key(
            &ctx.gpu,
            BindGroupLayoutCacheKey {
                entries: texture_entries,
            },
        )?;

        // samplers
        let texture_sampler_keys = ctx.textures.mega_texture_sampler_set.clone();

        if texture_sampler_keys.len() > device_limits.max_samplers_per_shader_stage() as usize {
            return Err(AwsmCoreError::MegaTextureTooManySamplers {
                total_samplers: texture_sampler_keys.len() as u32,
                max_samplers: device_limits.max_samplers_per_shader_stage(),
            }
            .into());
        }

        let mut sampler_entries = Vec::new();

        for _ in 0..texture_sampler_keys.len() {
            sampler_entries.push(BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Sampler(
                    SamplerBindingLayout::new().with_binding_type(SamplerBindingType::Filtering),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            });
        }

        let sampler_bind_group_layout_key = ctx.bind_group_layouts.get_key(
            &ctx.gpu,
            BindGroupLayoutCacheKey {
                entries: sampler_entries,
            },
        )?;

        Ok(Self {
            main_bind_group_layout_key,
            texture_bind_group_layout_key,
            sampler_bind_group_layout_key,
            texture_atlas_len,
            texture_sampler_keys,
            _main_bind_group: None,
            _texture_bind_group: None,
            _sampler_bind_group: None,
        })
    }

    pub fn get_bind_groups(
        &self,
    ) -> std::result::Result<
        (
            &web_sys::GpuBindGroup,
            &web_sys::GpuBindGroup,
            &web_sys::GpuBindGroup,
        ),
        AwsmBindGroupError,
    > {
        match (
            &self._main_bind_group,
            &self._texture_bind_group,
            &self._sampler_bind_group,
        ) {
            (Some(main_bind_group), Some(texture_bind_group), Some(sampler_bind_group)) => {
                Ok((main_bind_group, texture_bind_group, sampler_bind_group))
            }
            (None, Some(_), _) => Err(AwsmBindGroupError::NotFound(
                "Material Opaque - Main".to_string(),
            )),
            (Some(_), None, _) => Err(AwsmBindGroupError::NotFound(
                "Material Opaque - Texture".to_string(),
            )),
            (Some(_), Some(_), None) => Err(AwsmBindGroupError::NotFound(
                "Material Opaque - Sampler".to_string(),
            )),
            _ => Err(AwsmBindGroupError::NotFound("Material Opaque".to_string())),
        }
    }

    pub fn recreate_main(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut entries = Vec::new();

        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.meshes.meta.material_gpu_buffer())),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::TextureView(Cow::Borrowed(
                &ctx.render_texture_views.visibility_data,
            )),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::TextureView(Cow::Borrowed(&ctx.render_texture_views.opaque_color)),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(
                &ctx.materials.gpu_buffer(MaterialBufferKind::Pbr),
            )),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.meshes.attribute_index_gpu_buffer())),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.meshes.attribute_data_gpu_buffer())),
        ));
        // transforms
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.transforms.gpu_buffer)),
        ));
        // normal matrices
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.transforms.normals_gpu_buffer)),
        ));
        // camera
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.camera.gpu_buffer)),
        ));

        //skybox
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::TextureView(Cow::Borrowed(&ctx.environment.skybox.texture_view)),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Sampler(&ctx.environment.skybox.sampler),
        ));

        // IBL filtered env
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::TextureView(Cow::Borrowed(
                &ctx.lights.ibl.prefiltered_env.texture_view,
            )),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Sampler(&ctx.lights.ibl.prefiltered_env.sampler),
        ));

        // IBL irradiance
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::TextureView(Cow::Borrowed(&ctx.lights.ibl.irradiance.texture_view)),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Sampler(&ctx.lights.ibl.irradiance.sampler),
        ));

        // IBL info
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.lights.gpu_ibl_buffer)),
        ));

        // BRDF lut
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::TextureView(Cow::Borrowed(&ctx.lights.brdf_lut.view)),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Sampler(&ctx.lights.brdf_lut.sampler),
        ));

        // depth
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::TextureView(Cow::Borrowed(&ctx.render_texture_views.depth)),
        ));
        // visibility
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.meshes.visibility_data_gpu_buffer())),
        ));

        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts
                .get(self.main_bind_group_layout_key)?,
            Some("Material Opaque - Main"),
            entries,
        );

        self._main_bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }

    pub fn recreate_textures(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut entries = Vec::new();

        for i in 0..self.texture_atlas_len as usize {
            entries.push(BindGroupEntry::new(
                entries.len() as u32,
                BindGroupResource::TextureView(Cow::Borrowed(
                    &ctx.textures.gpu_texture_array_views[i],
                )),
            ));
        }

        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts
                .get(self.texture_bind_group_layout_key)?,
            Some("Material Opaque - Texture"),
            entries,
        );

        self._texture_bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }

    pub fn recreate_samplers(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut entries = Vec::new();

        for sampler_key in self.texture_sampler_keys.iter() {
            let sampler = ctx.textures.get_sampler(*sampler_key)?;

            entries.push(BindGroupEntry::new(
                entries.len() as u32,
                BindGroupResource::Sampler(sampler),
            ));
        }

        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts
                .get(self.sampler_bind_group_layout_key)?,
            Some("Material Opaque - Sampler"),
            entries,
        );

        self._sampler_bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }
}
