use std::{borrow::Cow, sync::LazyLock};

use crate::{
    bind_group_layout::{
        BindGroupLayoutCacheKey, BindGroupLayoutCacheKeyEntry, BindGroupLayoutKey,
    },
    bind_groups::{AwsmBindGroupError, BindGroupRecreateContext},
    error::Result,
    render_passes::{composite, RenderPassInitContext},
    textures::{SamplerCacheKey, SamplerKey},
};
use awsm_renderer_core::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutResource, BindGroupResource,
        SamplerBindingLayout, SamplerBindingType, TextureBindingLayout,
    },
    renderer::AwsmRendererWebGpu,
    sampler::{FilterMode, SamplerDescriptor},
    texture::{TextureSampleType, TextureViewDimension},
};

#[derive(Default)]
pub struct DisplayBindGroups {
    pub bind_group_layout_key: BindGroupLayoutKey,
    // this is set via `recreate` mechanism
    _bind_group: Option<web_sys::GpuBindGroup>,
}

impl DisplayBindGroups {
    pub async fn new(ctx: &mut RenderPassInitContext<'_>) -> Result<Self> {
        let bind_group_layout_cache_key = BindGroupLayoutCacheKey {
            entries: vec![BindGroupLayoutCacheKeyEntry {
                resource: BindGroupLayoutResource::Texture(
                    TextureBindingLayout::new()
                        .with_view_dimension(TextureViewDimension::N2d)
                        .with_sample_type(TextureSampleType::Float),
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

    pub fn get_bind_group(
        &self,
    ) -> std::result::Result<&web_sys::GpuBindGroup, AwsmBindGroupError> {
        self._bind_group
            .as_ref()
            .ok_or_else(|| AwsmBindGroupError::NotFound("Display".to_string()))
    }

    pub fn recreate(&mut self, ctx: &BindGroupRecreateContext<'_>) -> Result<()> {
        let descriptor = BindGroupDescriptor::new(
            ctx.bind_group_layouts.get(self.bind_group_layout_key)?,
            Some("Display"),
            vec![BindGroupEntry::new(
                0,
                BindGroupResource::TextureView(Cow::Borrowed(&ctx.render_texture_views.composite)),
            )],
        );

        self._bind_group = Some(ctx.gpu.create_bind_group(&descriptor.into()));

        Ok(())
    }
}
