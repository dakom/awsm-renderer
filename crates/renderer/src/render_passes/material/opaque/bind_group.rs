use std::borrow::Cow;

use awsm_renderer_core::bind_groups::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutResource, BindGroupResource,
    StorageTextureAccess, StorageTextureBindingLayout, TextureBindingLayout,
};
use awsm_renderer_core::texture::{TextureSampleType, TextureViewDimension};

use crate::bind_group_layout::{BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry};
use crate::bind_groups::{AwsmBindGroupError, BindGroupRecreateContext};
use crate::error::Result;
use crate::{bind_group_layout::BindGroupLayoutKey, render_passes::RenderPassInitContext};

pub struct MaterialOpaqueBindGroups {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_groups: Option<[web_sys::GpuBindGroup; 2]>,
}

impl MaterialOpaqueBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Texture(
                        TextureBindingLayout::new()
                            .with_view_dimension(TextureViewDimension::N2d)
                            .with_sample_type(TextureSampleType::Uint),
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
            ],
        };

        let bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(&ctx.gpu, bind_group_layout_cache_key)?;

        Ok(Self {
            bind_group_layout_key,
            _bind_groups: None,
        })
    }

    pub fn get_bind_group(
        &self,
        curr_index: usize,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_groups
            .as_ref()
            .map(|xs| &xs[curr_index])
            .ok_or_else(|| AwsmBindGroupError::NotFound("Material Opaque".to_string()))
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor_0 = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Material Opaque (0)"),
            vec![
                BindGroupEntry::new(
                    0,
                    BindGroupResource::TextureView(Cow::Borrowed(
                        &ctx.render_texture_views.material_offset,
                    )),
                ),
                BindGroupEntry::new(
                    1,
                    BindGroupResource::TextureView(Cow::Borrowed(
                        &ctx.render_texture_views.world_normal,
                    )),
                ),
                BindGroupEntry::new(
                    2,
                    BindGroupResource::TextureView(Cow::Borrowed(
                        &ctx.render_texture_views.screen_pos[0],
                    )),
                ),
                BindGroupEntry::new(
                    3,
                    BindGroupResource::TextureView(Cow::Borrowed(
                        &ctx.render_texture_views.opaque_color,
                    )),
                ),
            ],
        );

        let descriptor_1 = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Material Opaque (1)"),
            vec![
                BindGroupEntry::new(
                    0,
                    BindGroupResource::TextureView(Cow::Borrowed(
                        &ctx.render_texture_views.material_offset,
                    )),
                ),
                BindGroupEntry::new(
                    1,
                    BindGroupResource::TextureView(Cow::Borrowed(
                        &ctx.render_texture_views.world_normal,
                    )),
                ),
                BindGroupEntry::new(
                    2,
                    BindGroupResource::TextureView(Cow::Borrowed(
                        &ctx.render_texture_views.screen_pos[1],
                    )),
                ),
                BindGroupEntry::new(
                    3,
                    BindGroupResource::TextureView(Cow::Borrowed(
                        &ctx.render_texture_views.opaque_color,
                    )),
                ),
            ],
        );

        self._bind_groups = Some([
            ctx.gpu.create_bind_group(&descriptor_0.into()),
            ctx.gpu.create_bind_group(&descriptor_1.into()),
        ]);

        Ok(())
    }
}
