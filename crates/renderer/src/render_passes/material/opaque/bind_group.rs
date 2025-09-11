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

pub const MATERIAL_OPAQUE_CORE_TEXTURES_START_GROUP: u32 = 0;

pub struct MaterialOpaqueBindGroups {
    pub bind_group_layout_keys: Vec<BindGroupLayoutKey>,
    pub texture_bindings: MegaTextureBindings,
    // this is set via `recreate` mechanism
    _bind_groups: Option<Vec<web_sys::GpuBindGroup>>,
}

impl MaterialOpaqueBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let mut entries = vec![
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
            entries.len() as u32,
        );

        let mut bind_group_layout_keys = Vec::new();

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
    ) -> std::result::Result<&[web_sys::GpuBindGroup], AwsmBindGroupError> {
        self._bind_groups
            .as_deref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Material Opaque".to_string()))
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let mut entries = vec![
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
        ];

        let mut texture_count = 0;
        let mut bind_groups = Vec::new();
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
                Some("Material Opaque"),
                entries,
            );

            bind_groups.push(ctx.gpu.create_bind_group(&descriptor.into()));

            entries = Vec::new();
        }

        self._bind_groups = Some(bind_groups);
        Ok(())
    }
}
