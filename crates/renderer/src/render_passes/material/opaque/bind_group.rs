use std::borrow::Cow;

use awsm_renderer_core::bind_groups::{
    self, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutResource, BindGroupResource,
    BufferBindingLayout, BufferBindingType, SamplerBindingLayout, SamplerBindingType,
    StorageTextureAccess, StorageTextureBindingLayout, TextureBindingLayout,
};
use awsm_renderer_core::buffers::BufferBinding;
use awsm_renderer_core::renderer::AwsmRendererWebGpu;
use awsm_renderer_core::texture::mega_texture::MegaTextureBindings;
use awsm_renderer_core::texture::{self, TextureSampleType, TextureViewDimension};

use crate::bind_group_layout::{BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry};
use crate::bind_groups::{AwsmBindGroupError, BindGroupRecreateContext};
use crate::error::Result;
use crate::materials::pbr::PbrMaterial;
use crate::materials::MaterialBufferKind;
use crate::mesh::meta::material_opaque_meta::MATERIAL_MESH_META_BYTE_ALIGNMENT;
use crate::textures::SamplerBindings;
use crate::{bind_group_layout::BindGroupLayoutKey, render_passes::RenderPassInitContext};

pub const MATERIAL_OPAQUE_CORE_TEXTURES_START_GROUP: u32 = 0;

pub struct MaterialOpaqueBindGroups {
    pub bind_group_layout_keys: Vec<BindGroupLayoutKey>,
    pub texture_bindings: MegaTextureBindings,
    pub sampler_bindings: SamplerBindings,
    // this is set via `recreate` mechanism
    _bind_groups: Option<Vec<web_sys::GpuBindGroup>>,
}

impl MaterialOpaqueBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let base_entries = vec![
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
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
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
            BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::ReadOnlyStorage),
                ),
                visibility_vertex: false,
                visibility_fragment: false,
                visibility_compute: true,
            },
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

        let texture_bindings = ctx.textures.mega_texture.get_bindings(
            &ctx.gpu.device.limits(),
            MATERIAL_OPAQUE_CORE_TEXTURES_START_GROUP,
            base_entries.len() as u32,
        );

        let mut group_entries = vec![base_entries];

        for (group_index, len) in texture_bindings.bind_group_bindings_len.iter().enumerate() {
            if *len == 0 {
                continue;
            }

            if group_index == 0 {
                let entries = group_entries
                    .first_mut()
                    .expect("material opaque base bind group entries");
                for _ in 0..*len {
                    entries.push(BindGroupLayoutCacheKeyEntry {
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
            } else {
                let mut entries = Vec::new();
                for _ in 0..*len {
                    entries.push(BindGroupLayoutCacheKeyEntry {
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
                group_entries.push(entries);
            }
        }

        let sampler_start_group = texture_bindings.start_group + group_entries.len() as u32;
        let sampler_bindings =
            ctx.textures
                .sampler_bindings(&ctx.gpu.device.limits(), sampler_start_group, 0);

        for len in sampler_bindings.bind_group_bindings_len.iter() {
            if *len == 0 {
                continue;
            }

            let mut entries = Vec::new();
            for _ in 0..*len {
                entries.push(BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Sampler(
                        SamplerBindingLayout::new()
                            .with_binding_type(SamplerBindingType::Filtering),
                    ),
                    visibility_vertex: false,
                    visibility_fragment: false,
                    visibility_compute: true,
                });
            }
            // When more samplers exist than fit alongside the core bind group, additional entries
            // spill into successive groups. The shader template mirrors this arrangement to keep
            // sampler indices stable across pipeline recompiles.
            group_entries.push(entries);
        }

        let mut bind_group_layout_keys = Vec::new();
        for entries in group_entries {
            let bind_group_layout_key = ctx
                .bind_group_layouts
                .get_key(&ctx.gpu, BindGroupLayoutCacheKey { entries })?;

            bind_group_layout_keys.push(bind_group_layout_key);
        }

        Ok(Self {
            bind_group_layout_keys,
            texture_bindings,
            sampler_bindings,
            _bind_groups: None,
        })
    }

    pub fn get_bind_groups(
        &self,
    ) -> std::result::Result<&[web_sys::GpuBindGroup], AwsmBindGroupError> {
        self._bind_groups
            .as_deref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Material Opaque".to_string()))
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut bind_groups = Vec::new();
        let mut texture_index = 0usize;
        let mut sampler_index = 0usize;

        for (group_idx, layout_key) in self.bind_group_layout_keys.iter().enumerate() {
            let mut entries = Vec::new();

            if group_idx == 0 {
                // Group 0 always carries the shared buffers + visibility textures used by the
                // compute material pass. Extra texture/sampler bindings are appended below.
                entries.extend_from_slice(&[
                    BindGroupEntry::new(
                        0,
                        BindGroupResource::Buffer(BufferBinding::new(
                            &ctx.meshes.meta.material_gpu_buffer(),
                        )),
                    ),
                    BindGroupEntry::new(
                        1,
                        BindGroupResource::TextureView(Cow::Borrowed(
                            &ctx.render_texture_views.visibility_data,
                        )),
                    ),
                    BindGroupEntry::new(
                        2,
                        BindGroupResource::TextureView(Cow::Borrowed(
                            &ctx.render_texture_views.opaque_color,
                        )),
                    ),
                    BindGroupEntry::new(
                        3,
                        BindGroupResource::Buffer(BufferBinding::new(
                            &ctx.materials.gpu_buffer(MaterialBufferKind::Pbr),
                        )),
                    ),
                    BindGroupEntry::new(
                        4,
                        BindGroupResource::Buffer(BufferBinding::new(
                            &ctx.meshes.attribute_index_gpu_buffer(),
                        )),
                    ),
                    BindGroupEntry::new(
                        5,
                        BindGroupResource::Buffer(BufferBinding::new(
                            &ctx.meshes.attribute_data_gpu_buffer(),
                        )),
                    ),
                ]);
            }

            let group_idx_u32 = group_idx as u32;

            if group_idx_u32 >= self.texture_bindings.start_group
                && group_idx_u32
                    < self.texture_bindings.start_group
                        + self.texture_bindings.bind_group_bindings_len.len() as u32
            {
                let texture_group_index =
                    (group_idx_u32 - self.texture_bindings.start_group) as usize;
                let len = self.texture_bindings.bind_group_bindings_len[texture_group_index];
                for _ in 0..len {
                    entries.push(BindGroupEntry::new(
                        entries.len() as u32,
                        BindGroupResource::TextureView(Cow::Borrowed(
                            &ctx.textures.gpu_texture_array_views[texture_index],
                        )),
                    ));
                    texture_index += 1;
                }
            }

            if !self.sampler_bindings.bind_group_bindings_len.is_empty()
                && group_idx_u32 >= self.sampler_bindings.start_group
                && group_idx_u32
                    < self.sampler_bindings.start_group
                        + self.sampler_bindings.bind_group_bindings_len.len() as u32
            {
                let sampler_group_index =
                    (group_idx_u32 - self.sampler_bindings.start_group) as usize;
                let len = self.sampler_bindings.bind_group_bindings_len[sampler_group_index];
                for _ in 0..len {
                    let sampler_key = ctx.textures.sampler_keys()[sampler_index];
                    let sampler = ctx.textures.get_sampler(sampler_key)?;
                    entries.push(BindGroupEntry::new(
                        entries.len() as u32,
                        BindGroupResource::Sampler(sampler),
                    ));
                    sampler_index += 1;
                }
            }

            let descriptor = BindGroupDescriptor::new(
                ctx.bind_group_layouts.get(*layout_key)?,
                Some("Material Opaque"),
                entries,
            );

            bind_groups.push(ctx.gpu.create_bind_group(&descriptor.into()));
        }

        self._bind_groups = Some(bind_groups);

        Ok(())
    }
}
