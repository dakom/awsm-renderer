use std::borrow::Cow;

use awsm_renderer_core::bind_groups::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutResource, BindGroupResource,
    StorageTextureAccess, StorageTextureBindingLayout, TextureBindingLayout,
};
use awsm_renderer_core::texture::{TextureSampleType, TextureViewDimension};

use crate::anti_alias::AntiAliasing;
use crate::bind_group_layout::{
    BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry, BindGroupLayoutKey,
};
use crate::bind_groups::{AwsmBindGroupError, BindGroupRecreateContext};
use crate::error::Result;
use crate::render_passes::RenderPassInitContext;

#[derive(Default)]
pub struct CompositeBindGroups {
    pub multisampled_bind_group_layout_key: BindGroupLayoutKey,
    pub singlesampled_bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl CompositeBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let multisampled_bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::Texture(
                        TextureBindingLayout::new()
                            .with_view_dimension(TextureViewDimension::N2d)
                            .with_sample_type(TextureSampleType::UnfilterableFloat)
                            .with_multisampled(true),
                    ),
                    visibility_vertex: false,
                    visibility_fragment: false,
                    visibility_compute: true,
                },
                BindGroupLayoutCacheKeyEntry {
                    resource: BindGroupLayoutResource::StorageTexture(
                        StorageTextureBindingLayout::new(ctx.render_texture_formats.composite)
                            .with_view_dimension(TextureViewDimension::N2d)
                            .with_access(StorageTextureAccess::WriteOnly),
                    ),
                    visibility_vertex: false,
                    visibility_fragment: false,
                    visibility_compute: true,
                },
            ],
        };

        let singlesampled_bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![
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
                        StorageTextureBindingLayout::new(ctx.render_texture_formats.composite)
                            .with_view_dimension(TextureViewDimension::N2d)
                            .with_access(StorageTextureAccess::WriteOnly),
                    ),
                    visibility_vertex: false,
                    visibility_fragment: false,
                    visibility_compute: true,
                },
            ],
        };

        let multisampled_bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(&ctx.gpu, multisampled_bind_group_layout_cache_key)?;

        let singlesampled_bind_group_layout_key = ctx
            .bind_group_layouts
            .get_key(&ctx.gpu, singlesampled_bind_group_layout_cache_key)?;

        Ok(Self {
            multisampled_bind_group_layout_key,
            singlesampled_bind_group_layout_key,
            _bind_group: None,
        })
    }

    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Composite".to_string()))
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let entries = vec![
            BindGroupEntry::new(
                0,
                BindGroupResource::TextureView(Cow::Borrowed(
                    &ctx.render_texture_views.transparent,
                )),
            ),
            BindGroupEntry::new(
                1,
                BindGroupResource::TextureView(Cow::Borrowed(&ctx.render_texture_views.composite)),
            ),
        ];

        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts
                .get(if ctx.anti_aliasing.msaa_sample_count.is_some() {
                    self.multisampled_bind_group_layout_key
                } else {
                    self.singlesampled_bind_group_layout_key
                })?,
            Some("Composite"),
            entries.clone(),
        );

        self._bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }
}
