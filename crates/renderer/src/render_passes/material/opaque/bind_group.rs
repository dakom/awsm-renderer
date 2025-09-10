use std::borrow::Cow;

use awsm_renderer_core::bind_groups::{
    self, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutResource, BindGroupResource,
    BufferBindingLayout, BufferBindingType, StorageTextureAccess, StorageTextureBindingLayout,
    TextureBindingLayout,
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
use crate::mesh::meta::MATERIAL_MESH_META_BYTE_ALIGNMENT;
use crate::{bind_group_layout::BindGroupLayoutKey, render_passes::RenderPassInitContext};

const TEXTURES_START_GROUP: u32 = 2; // after core and meta
const TEXTURES_START_GROUP_BINDING: u32 = 0; // first binding in the group

pub struct MaterialOpaqueBindGroups {
    pub core: MaterialCoreBindGroup,
    pub meta: MaterialMetaBindGroup,
    pub textures: MaterialTextureBindGroups,
}

impl MaterialOpaqueBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let core = MaterialCoreBindGroup::new(ctx).await?;
        let meta = MaterialMetaBindGroup::new(ctx).await?;
        let textures = MaterialTextureBindGroups::new(ctx).await?;

        Ok(Self {
            core,
            meta,
            textures,
        })
    }

    pub fn all_layout_keys(&self) -> Vec<BindGroupLayoutKey> {
        let mut keys = vec![
            self.core.bind_group_layout_key,
            self.meta.bind_group_layout_key,
        ];
        keys.extend(self.textures.bind_group_layout_keys.iter());
        keys
    }
}

pub struct MaterialCoreBindGroup {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl MaterialCoreBindGroup {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let mut entries = vec![
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

        let bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(&ctx.gpu, BindGroupLayoutCacheKey { entries })?;

        Ok(Self {
            bind_group_layout_key,
            _bind_group: None,
        })
    }

    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Material Opaque - Core".to_string()))
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut entries = vec![
            BindGroupEntry::new(
                0,
                BindGroupResource::TextureView(Cow::Borrowed(
                    &ctx.render_texture_views.visibility_data,
                )),
            ),
            BindGroupEntry::new(
                1,
                BindGroupResource::TextureView(Cow::Borrowed(
                    &ctx.render_texture_views.opaque_color,
                )),
            ),
            BindGroupEntry::new(
                2,
                BindGroupResource::Buffer(BufferBinding::new(
                    &ctx.materials.gpu_buffer(MaterialBufferKind::Pbr),
                )),
            ),
            BindGroupEntry::new(
                3,
                BindGroupResource::Buffer(BufferBinding::new(
                    &ctx.meshes.attribute_index_gpu_buffer(),
                )),
            ),
            BindGroupEntry::new(
                4,
                BindGroupResource::Buffer(BufferBinding::new(
                    &ctx.meshes.attribute_data_gpu_buffer(),
                )),
            ),
        ];

        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Material Opaque - Core"),
            entries,
        );

        self._bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));
        Ok(())
    }
}

pub struct MaterialTextureBindGroups {
    pub bind_group_layout_keys: Vec<BindGroupLayoutKey>,
    pub texture_bindings: MegaTextureBindings,
    // this is set via `recreate` mechanism
    _bind_groups: Option<Vec<web_sys::GpuBindGroup>>,
}

impl MaterialTextureBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let texture_bindings = ctx.textures.mega_texture.get_bindings(
            &ctx.gpu.device.limits(),
            TEXTURES_START_GROUP,
            TEXTURES_START_GROUP_BINDING,
        );

        let mut bind_group_layout_keys = Vec::new();

        let mut entries = Vec::new();
        for len in texture_bindings.bind_group_bindings_len.iter() {
            for i in 0..*len {
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

            let bind_group_layout_key = ctx
                .bind_group_layouts
                .get_key(&ctx.gpu, BindGroupLayoutCacheKey { entries })?;

            bind_group_layout_keys.push(bind_group_layout_key);

            entries = Vec::new();
        }

        Ok(Self {
            bind_group_layout_keys,
            texture_bindings,
            _bind_groups: None,
        })
    }

    pub fn get_bind_groups(
        &self,
    ) -> std::result::Result<&Vec<web_sys::GpuBindGroup>, AwsmBindGroupError> {
        self._bind_groups
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Material Opaque - Textures".to_string()))
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut texture_count = 0;
        let mut bind_groups = Vec::new();
        let mut entries = Vec::new();

        for len in self.texture_bindings.bind_group_bindings_len.iter() {
            for i in 0..*len {
                entries.push(BindGroupEntry::new(
                    entries.len() as u32,
                    BindGroupResource::TextureView(Cow::Borrowed(
                        &ctx.textures.gpu_texture_array_views[texture_count],
                    )),
                ));

                texture_count += 1;
            }

            let descriptor = BindGroupDescriptor::new(
                ctx.bind_group_layouts
                    .get(self.bind_group_layout_keys[bind_groups.len()])?,
                Some("Material Opaque - Textures"),
                entries,
            );

            bind_groups.push(ctx.gpu.create_bind_group(&descriptor.into()));

            entries = Vec::new();
        }

        self._bind_groups = Some(bind_groups);
        Ok(())
    }
}

#[derive(Default)]
pub struct MaterialMetaBindGroup {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl MaterialMetaBindGroup {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Buffer(
                    BufferBindingLayout::new()
                        .with_binding_type(BufferBindingType::Uniform)
                        .with_dynamic_offset(true),
                ),
                visibility_vertex: true,
                visibility_fragment: true,
                visibility_compute: false,
            }],
        };

        let bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(&ctx.gpu, bind_group_layout_cache_key)?;

        Ok(Self {
            bind_group_layout_key,
            _bind_group: None,
        })
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Material Opaque - Meta"),
            vec![BindGroupEntry::new(
                0,
                BindGroupResource::Buffer(
                    BufferBinding::new(&ctx.meshes.meta.material_gpu_buffer())
                        .with_size(MATERIAL_MESH_META_BYTE_ALIGNMENT),
                ),
            )],
        );

        let bind_group = ctx.gpu.create_bind_group(&descriptor.into());
        self._bind_group = Some(bind_group);

        Ok(())
    }

    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Material Opaque - Meta".to_string()))
    }
}
