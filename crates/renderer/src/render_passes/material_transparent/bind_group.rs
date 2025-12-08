use std::borrow::Cow;

use awsm_renderer_core::bind_groups::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutResource, BindGroupResource,
    BufferBindingLayout, BufferBindingType, SamplerBindingLayout, SamplerBindingType,
    TextureBindingLayout,
};
use awsm_renderer_core::buffers::BufferBinding;
use awsm_renderer_core::texture::TextureViewDimension;
use indexmap::IndexSet;

use crate::bind_group_layout::{BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry};
use crate::bind_groups::{AwsmBindGroupError, BindGroupRecreateContext};
use crate::error::Result;
use crate::materials::MaterialBufferKind;
use crate::mesh::meta::geometry_meta::GEOMETRY_MESH_META_BYTE_ALIGNMENT;
use crate::mesh::meta::material_meta::MATERIAL_MESH_META_BYTE_ALIGNMENT;
use crate::render_passes::shared::opaque_and_transparency::bind_group::{
    TexturePoolDeps, TexturePoolVisibility,
};
use crate::textures::SamplerKey;
use crate::{bind_group_layout::BindGroupLayoutKey, render_passes::RenderPassInitContext};

pub struct MaterialTransparentBindGroups {
    pub main_bind_group_layout_key: BindGroupLayoutKey,
    pub mesh_material_bind_group_layout_key: BindGroupLayoutKey,
    pub lights_bind_group_layout_key: BindGroupLayoutKey,
    pub texture_pool_textures_bind_group_layout_key: BindGroupLayoutKey,
    pub texture_pool_arrays_len: u32,
    pub texture_pool_sampler_keys: IndexSet<SamplerKey>,

    _main_bind_group: Option<web_sys::GpuBindGroup>,
    _mesh_material_bind_group: Option<web_sys::GpuBindGroup>,
    _lights_bind_group: Option<web_sys::GpuBindGroup>,
    _texture_bind_group: Option<web_sys::GpuBindGroup>,
}

impl MaterialTransparentBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let TexturePoolDeps {
            bind_group_layout_key: texture_pool_textures_bind_group_layout_key,
            arrays_len: texture_pool_arrays_len,
            sampler_keys: texture_pool_sampler_keys,
        } = TexturePoolDeps::new(ctx, TexturePoolVisibility::Render)?;

        let entries = vec![
            // Camera
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new().with_binding_type(BufferBindingType::Uniform),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
            // Transform
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
            // Materials
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
            // Morph weights
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
            // Morph values
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
            // Skin matrices
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
            // Skin weights
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
            // Texture transforms
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            },
        ];

        let main_bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(ctx.gpu, BindGroupLayoutCacheKey { entries })?;

        // Mesh meta
        let mesh_material_bind_group_layout_key = ctx.bind_group_layouts.get_key(
            ctx.gpu,
            BindGroupLayoutCacheKey {
                entries: vec![
                    // GeometryMeshMeta
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Buffer(
                            BufferBindingLayout::new()
                                .with_binding_type(BufferBindingType::Uniform)
                                .with_dynamic_offset(true),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                    // MaterialMeshMeta
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Buffer(
                            BufferBindingLayout::new()
                                .with_binding_type(BufferBindingType::Uniform)
                                .with_dynamic_offset(true),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                ],
            },
        )?;

        // lights

        let lights_bind_group_layout_key = ctx.bind_group_layouts.get_key(
            ctx.gpu,
            BindGroupLayoutCacheKey {
                entries: vec![
                    // info
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Buffer(
                            BufferBindingLayout::new()
                                .with_binding_type(BufferBindingType::Uniform),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                    // punctual lights
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Buffer(
                            BufferBindingLayout::new()
                                .with_binding_type(BufferBindingType::ReadOnlyStorage),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                    // IBL prefiltered env texture
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Texture(
                            TextureBindingLayout::new()
                                .with_view_dimension(TextureViewDimension::Cube),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                    // IBL prefiltered env sampler
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Sampler(
                            SamplerBindingLayout::new()
                                .with_binding_type(SamplerBindingType::Filtering),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                    // IBL irradiance env texture
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Texture(
                            TextureBindingLayout::new()
                                .with_view_dimension(TextureViewDimension::Cube),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                    // IBL irradiance env sampler
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Sampler(
                            SamplerBindingLayout::new()
                                .with_binding_type(SamplerBindingType::Filtering),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                    // Brdf lut texture
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Texture(
                            TextureBindingLayout::new()
                                .with_view_dimension(TextureViewDimension::N2d),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                    // Brdf lut sampler
                    BindGroupLayoutCacheKeyEntry {
                        resource: BindGroupLayoutResource::Sampler(
                            SamplerBindingLayout::new()
                                .with_binding_type(SamplerBindingType::Filtering),
                        ),
                        visibility_vertex: true,
                        visibility_fragment: true,
                        visibility_compute: false,
                    },
                ],
            },
        )?;

        // Texture Pool

        Ok(Self {
            main_bind_group_layout_key,
            mesh_material_bind_group_layout_key,
            lights_bind_group_layout_key,

            texture_pool_textures_bind_group_layout_key,
            texture_pool_arrays_len,
            texture_pool_sampler_keys,

            _main_bind_group: None,
            _mesh_material_bind_group: None,
            _lights_bind_group: None,
            _texture_bind_group: None,
        })
    }

    pub fn clone_because_texture_pool_changed(
        &self,
        ctx: &mut RenderPassInitContext<'_>,
    ) -> Result<Self> {
        let TexturePoolDeps {
            bind_group_layout_key: texture_pool_textures_bind_group_layout_key,
            arrays_len: texture_pool_arrays_len,
            sampler_keys: texture_pool_sampler_keys,
        } = TexturePoolDeps::new(ctx, TexturePoolVisibility::Render)?;

        let mut _self = Self {
            main_bind_group_layout_key: self.main_bind_group_layout_key,
            mesh_material_bind_group_layout_key: self.mesh_material_bind_group_layout_key,
            lights_bind_group_layout_key: self.lights_bind_group_layout_key,
            texture_pool_textures_bind_group_layout_key,
            texture_pool_arrays_len,
            texture_pool_sampler_keys,
            _main_bind_group: self._main_bind_group.clone(),
            _mesh_material_bind_group: self._mesh_material_bind_group.clone(),
            _lights_bind_group: self._lights_bind_group.clone(),
            _texture_bind_group: None,
        };

        Ok(_self)
    }

    pub fn get_bind_groups(
        &self,
    ) -> std::result::Result<
        (
            &web_sys::GpuBindGroup,
            &web_sys::GpuBindGroup,
            &web_sys::GpuBindGroup,
            &web_sys::GpuBindGroup,
        ),
        AwsmBindGroupError,
    > {
        match (
            &self._main_bind_group,
            &self._mesh_material_bind_group,
            &self._lights_bind_group,
            &self._texture_bind_group,
        ) {
            (
                Some(main_bind_group),
                Some(mesh_material_bind_group),
                Some(lights_bind_group),
                Some(texture_bind_group),
            ) => Ok((
                main_bind_group,
                mesh_material_bind_group,
                lights_bind_group,
                texture_bind_group,
            )),
            (None, _, _, _) => Err(AwsmBindGroupError::NotFound(
                "Material Transparent - Main".to_string(),
            )),
            (_, None, _, _) => Err(AwsmBindGroupError::NotFound(
                "Material Transparent - Mesh Material".to_string(),
            )),
            (_, _, None, _) => Err(AwsmBindGroupError::NotFound(
                "Material Transparent - Lights".to_string(),
            )),
            (_, _, _, None) => Err(AwsmBindGroupError::NotFound(
                "Material Transparent - Texture Pool".to_string(),
            )),
        }
    }

    pub fn recreate_main(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut entries = Vec::new();

        // camera
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.camera.gpu_buffer)),
        ));

        // transform
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.transforms.gpu_buffer)),
        ));

        // materials
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(
                ctx.materials.gpu_buffer(MaterialBufferKind::Pbr),
            )),
        ));

        // morph weights
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(
                &ctx.meshes.morphs.geometry.gpu_buffer_weights,
            )),
        ));
        // morph values
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(
                &ctx.meshes.morphs.geometry.gpu_buffer_values,
            )),
        ));
        // skin matrices
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.meshes.skins.matrices_gpu_buffer)),
        ));
        // skin weights
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(
                &ctx.meshes.skins.joint_index_weights_gpu_buffer,
            )),
        ));
        // texture transofmrs
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(
                &ctx.textures.texture_transforms_gpu_buffer,
            )),
        ));

        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts
                .get(self.main_bind_group_layout_key)?,
            Some("Material Transparent - Main"),
            entries,
        );

        self._main_bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }

    pub fn recreate_mesh_material(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut entries = Vec::new();

        // geometry meta
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(
                BufferBinding::new(ctx.meshes.meta.geometry_gpu_buffer())
                    .with_size(GEOMETRY_MESH_META_BYTE_ALIGNMENT),
            ),
        ));

        // material meta
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(
                BufferBinding::new(ctx.meshes.meta.material_gpu_buffer())
                    .with_size(MATERIAL_MESH_META_BYTE_ALIGNMENT),
            ),
        ));

        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts
                .get(self.mesh_material_bind_group_layout_key)?,
            Some("Material Transparent - Mesh Material"),
            entries,
        );

        self._mesh_material_bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }

    pub fn recreate_lights(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut entries = Vec::new();

        // Lights info
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.lights.gpu_info_buffer)),
        ));

        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Buffer(BufferBinding::new(&ctx.lights.gpu_punctual_buffer)),
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

        // BRDF lut
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::TextureView(Cow::Borrowed(&ctx.lights.brdf_lut.view)),
        ));
        entries.push(BindGroupEntry::new(
            entries.len() as u32,
            BindGroupResource::Sampler(&ctx.lights.brdf_lut.sampler),
        ));

        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts
                .get(self.lights_bind_group_layout_key)?,
            Some("Material Transparent - Lights"),
            entries,
        );

        self._lights_bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }

    pub fn recreate_texture_pool(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut entries = Vec::new();

        for view in ctx.textures.pool.texture_views() {
            entries.push(BindGroupEntry::new(
                entries.len() as u32,
                BindGroupResource::TextureView(Cow::Borrowed(view)),
            ));
        }

        for sampler_key in self.texture_pool_sampler_keys.iter() {
            let sampler = ctx.textures.get_sampler(*sampler_key)?;

            entries.push(BindGroupEntry::new(
                entries.len() as u32,
                BindGroupResource::Sampler(sampler),
            ));
        }

        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts
                .get(self.texture_pool_textures_bind_group_layout_key)?,
            Some("Material Transparent - Texture Pool"),
            entries,
        );

        self._texture_bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }
}
