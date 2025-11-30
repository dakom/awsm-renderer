use awsm_renderer_core::{
    error::AwsmCoreError,
    pipeline::fragment::ColorTargetState,
    renderer::AwsmRendererWebGpu,
    sampler::SamplerDescriptor,
    texture::{
        blit::{blit_get_pipeline, BlitPipeline},
        clear::TextureClearer,
        Extent3d, TextureDescriptor, TextureFormat, TextureUsage,
    },
};
use thiserror::Error;

use crate::anti_alias::AntiAliasing;

pub struct RenderTextures {
    pub formats: RenderTextureFormats,
    pub opaque_to_transparent_blit_pipeline_msaa_4: BlitPipeline,
    pub opaque_to_transparent_blit_pipeline_no_anti_alias: BlitPipeline,
    frame_count: u32,
    inner: Option<RenderTexturesInner>,
}

#[derive(Clone, Debug)]
pub struct RenderTextureFormats {
    // Output from geometry pass
    pub visiblity_data: TextureFormat,
    pub barycentric: TextureFormat,
    pub normal_tangent: TextureFormat, // Packed: octahedral normal + tangent angle + handedness
    pub barycentric_derivatives: TextureFormat,

    // Output from opaque shading pass
    pub opaque: TextureFormat,

    // Output from transparent shading pass
    pub transparent: TextureFormat,

    // Output from composite pass
    pub composite: TextureFormat,

    // output from display pass is whatever current gpu texture format is

    // For depth testing and transparency
    pub depth: TextureFormat,
    // note - output from the composite pass will be whatever the gpu texture format is
}

impl RenderTextureFormats {
    pub async fn new(device: &web_sys::GpuDevice) -> Self {
        Self {
            visiblity_data: TextureFormat::Rgba16uint,
            barycentric: TextureFormat::Rg16float,
            normal_tangent: TextureFormat::Rgba16float,
            barycentric_derivatives: TextureFormat::Rgba16float,
            opaque: TextureFormat::Rgba16float, // HDR format for bloom/tonemapping
            transparent: TextureFormat::Rgba16float, // HDR format for bloom/tonemapping
            composite: TextureFormat::Rgba16float, // Final composite output format
            depth: TextureFormat::Depth24plus,  // Depth format for depth testing
        }
    }
}

impl RenderTextures {
    pub async fn new(gpu: &AwsmRendererWebGpu, formats: RenderTextureFormats) -> Result<Self> {
        let opaque_to_transparent_blit_pipeline_no_anti_alias =
            blit_get_pipeline(gpu, formats.transparent, None)
                .await
                .map_err(AwsmRenderTextureError::BlitPipeline)?;

        let opaque_to_transparent_blit_pipeline_msaa_4 =
            blit_get_pipeline(gpu, formats.transparent, Some(4))
                .await
                .map_err(AwsmRenderTextureError::BlitPipeline)?;

        Ok(Self {
            formats,
            frame_count: 0,
            inner: None,
            opaque_to_transparent_blit_pipeline_msaa_4,
            opaque_to_transparent_blit_pipeline_no_anti_alias,
        })
    }

    pub fn next_frame(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn ping_pong(&self) -> bool {
        self.frame_count() % 2 == 0
    }

    pub fn views(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        anti_aliasing: AntiAliasing,
    ) -> Result<RenderTextureViews> {
        let current_size = gpu
            .current_context_texture_size()
            .map_err(AwsmRenderTextureError::CurrentScreenSize)?;

        let size_changed = match self.inner.as_ref() {
            Some(inner) => (inner.width, inner.height) != current_size,
            None => true,
        };

        let anti_aliasing_changed = match self.inner.as_ref() {
            Some(inner) => inner.anti_aliasing != anti_aliasing,
            None => false,
        };

        if size_changed {
            if let Some(inner) = self.inner.take() {
                inner.destroy();
            }

            let inner = RenderTexturesInner::new(
                gpu,
                self.formats.clone(),
                current_size.0,
                current_size.1,
                anti_aliasing,
            )?;
            self.inner = Some(inner);
        }

        Ok(RenderTextureViews::new(
            self.inner.as_ref().unwrap(),
            self.ping_pong(),
            current_size.0,
            current_size.1,
            size_changed,
        ))
    }

    pub fn clear_opaque(&self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        if let Some(inner) = self.inner.as_ref() {
            inner
                .opaque_clearer
                .clear(gpu, &inner.opaque)
                .map_err(AwsmRenderTextureError::TextureClearerClear)
        } else {
            Ok(())
        }
    }
}

pub struct RenderTextureViews {
    // Output from geometry pass
    pub visibility_data: web_sys::GpuTextureView,
    pub barycentric: web_sys::GpuTextureView,
    pub normal_tangent: web_sys::GpuTextureView,
    pub barycentric_derivatives: web_sys::GpuTextureView,

    // Output from opaque pass
    pub opaque: web_sys::GpuTextureView,

    // Output from transparent pass
    pub transparent: web_sys::GpuTextureView,

    // Output from composite pass
    pub composite: web_sys::GpuTextureView,

    pub depth: web_sys::GpuTextureView,
    pub size_changed: bool,
    pub width: u32,
    pub height: u32,
    pub curr_index: usize,
    pub prev_index: usize,
}

impl RenderTextureViews {
    pub fn new(
        inner: &RenderTexturesInner,
        ping_pong: bool,
        width: u32,
        height: u32,
        size_changed: bool,
    ) -> Self {
        let curr_index = if ping_pong { 0 } else { 1 };
        let prev_index = if ping_pong { 1 } else { 0 };
        Self {
            visibility_data: inner.visibility_data_view.clone(),
            barycentric: inner.barycentric_view.clone(),
            normal_tangent: inner.normal_tangent_view.clone(),
            barycentric_derivatives: inner.barycentric_derivatives_view.clone(),
            opaque: inner.opaque_view.clone(),
            transparent: inner.transparent_view.clone(),
            depth: inner.depth_view.clone(),
            composite: inner.composite_view.clone(),
            size_changed,
            curr_index,
            prev_index,
            width,
            height,
        }
    }
}

#[allow(dead_code)]
pub struct RenderTexturesInner {
    pub visibility_data: web_sys::GpuTexture,
    pub visibility_data_view: web_sys::GpuTextureView,

    pub barycentric: web_sys::GpuTexture,
    pub barycentric_view: web_sys::GpuTextureView,

    // pub taa_clip_positions: [web_sys::GpuTexture; 2],
    // pub taa_clip_position_views: [web_sys::GpuTextureView; 2],
    pub normal_tangent: web_sys::GpuTexture,
    pub normal_tangent_view: web_sys::GpuTextureView,

    pub barycentric_derivatives: web_sys::GpuTexture,
    pub barycentric_derivatives_view: web_sys::GpuTextureView,

    pub opaque: web_sys::GpuTexture,
    pub opaque_clearer: TextureClearer,
    pub opaque_view: web_sys::GpuTextureView,

    pub transparent: web_sys::GpuTexture,
    pub transparent_view: web_sys::GpuTextureView,

    pub depth: web_sys::GpuTexture,
    pub depth_view: web_sys::GpuTextureView,

    pub composite: web_sys::GpuTexture,
    pub composite_view: web_sys::GpuTextureView,

    pub width: u32,
    pub height: u32,

    pub anti_aliasing: AntiAliasing,
}

impl RenderTexturesInner {
    pub fn new(
        gpu: &AwsmRendererWebGpu,
        render_texture_formats: RenderTextureFormats,
        width: u32,
        height: u32,
        anti_aliasing: AntiAliasing,
    ) -> Result<Self> {
        let maybe_multisample_texture =
            |format: TextureFormat, label: &'static str| -> TextureDescriptor<'static> {
                let mut descriptor = TextureDescriptor::new(
                    format,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label(label);

                if let Some(sample_count) = anti_aliasing.msaa_sample_count {
                    descriptor = descriptor.with_sample_count(sample_count);
                }

                descriptor
            };

        // 1. Create all textures
        let visibility_data = gpu
            .create_texture(
                &maybe_multisample_texture(
                    render_texture_formats.visiblity_data,
                    "Visibility Data",
                )
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let barycentric = gpu
            .create_texture(
                &maybe_multisample_texture(render_texture_formats.barycentric, "Barycentric")
                    .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        // let taa_clip_positions = [
        //     gpu.create_texture(
        //         &geometry_texture(
        //             render_texture_formats.taa_clip_position,
        //             "TAA Clip Position (0)",
        //         )
        //         .into(),
        //     )
        //     .map_err(AwsmRenderTextureError::CreateTexture)?,
        //     gpu.create_texture(
        //         &geometry_texture(
        //             render_texture_formats.taa_clip_position,
        //             "TAA Clip Position (1)",
        //         )
        //         .into(),
        //     )
        //     .map_err(AwsmRenderTextureError::CreateTexture)?,
        // ];

        let normal_tangent = gpu
            .create_texture(
                &maybe_multisample_texture(render_texture_formats.normal_tangent, "Normal Tangent")
                    .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let barycentric_derivatives = gpu
            .create_texture(
                &maybe_multisample_texture(
                    render_texture_formats.barycentric_derivatives,
                    "Barycentric Derivatives",
                )
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let opaque = gpu
            .create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.opaque,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_storage_binding()
                        .with_texture_binding()
                        .with_render_attachment()
                        .with_copy_dst(),
                )
                .with_label("Opaque")
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let transparent = gpu
            .create_texture(
                &maybe_multisample_texture(render_texture_formats.transparent, "Transparent")
                    .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let depth = gpu
            .create_texture(
                &maybe_multisample_texture(render_texture_formats.depth, "Depth").into(),
            )
            // Keeping the depth buffer bindable allows later passes (e.g. compute shading) to
            // sample it directly for world-position reconstruction.
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let composite = gpu
            .create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.composite,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_storage_binding()
                        .with_texture_binding(),
                )
                .with_label("Composite")
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        // 2. Create views for all textures

        let visibility_data_view = visibility_data.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("visibility_data: {e:?}"))
        })?;

        let barycentric_view = barycentric.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("barycentric: {e:?}"))
        })?;

        // let taa_clip_position_views = [
        //     taa_clip_positions[0].create_view().map_err(|e| {
        //         AwsmRenderTextureError::CreateTextureView(format!("taa_clip_positions[0]: {e:?}"))
        //     })?,
        //     taa_clip_positions[1].create_view().map_err(|e| {
        //         AwsmRenderTextureError::CreateTextureView(format!("taa_clip_positions[1]: {e:?}"))
        //     })?,
        // ];

        let normal_tangent_view = normal_tangent.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("normal_tangent: {e:?}"))
        })?;

        let barycentric_derivatives_view = barycentric_derivatives.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("barycentric: {e:?}"))
        })?;

        let opaque_view = opaque
            .create_view()
            .map_err(|e| AwsmRenderTextureError::CreateTextureView(format!("opaque: {e:?}")))?;

        let transparent_view = transparent.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("transparent: {e:?}"))
        })?;

        let depth_view = depth
            .create_view()
            .map_err(|e| AwsmRenderTextureError::CreateTextureView(format!("depth: {e:?}")))?;

        let composite_view = composite
            .create_view()
            .map_err(|e| AwsmRenderTextureError::CreateTextureView(format!("composite: {e:?}")))?;

        Ok(Self {
            visibility_data,
            visibility_data_view,

            barycentric,
            barycentric_view,

            normal_tangent,
            normal_tangent_view,

            barycentric_derivatives,
            barycentric_derivatives_view,

            opaque,
            opaque_view,
            opaque_clearer: TextureClearer::new(gpu, render_texture_formats.opaque, width, height)
                .map_err(AwsmRenderTextureError::CreateTextureClearer)?,

            transparent,
            transparent_view,

            depth,
            depth_view,

            composite,
            composite_view,

            width,
            height,

            anti_aliasing,
        })
    }

    pub fn destroy(self) {
        self.visibility_data.destroy();
        self.barycentric.destroy();
        // for texture in self.taa_clip_positions {
        //     texture.destroy();
        // }
        self.normal_tangent.destroy();
        self.barycentric_derivatives.destroy();
        self.opaque.destroy();
        self.transparent.destroy();
        self.depth.destroy();
        self.composite.destroy();
    }
}

type Result<T> = std::result::Result<T, AwsmRenderTextureError>;
#[derive(Debug, Error)]
pub enum AwsmRenderTextureError {
    #[error("[render_texture] Error creating texture: {0:?}")]
    CreateTexture(AwsmCoreError),

    #[error("[render_texture] Error creating texture view: {0}")]
    CreateTextureView(String),

    #[error("[render_texture] Error getting current screen size: {0:?}")]
    CurrentScreenSize(AwsmCoreError),

    #[error("[render_texture] Error getting current texture view: {0:?}")]
    CurrentTextureView(AwsmCoreError),

    #[error("[render_texture] Error creating texture clearer: {0:?}")]
    CreateTextureClearer(AwsmCoreError),

    #[error("[render_texture] Error clearing texture: {0:?}")]
    TextureClearerClear(AwsmCoreError),

    #[error("[render_texture] Blit pipeline: {0:?}")]
    BlitPipeline(AwsmCoreError),
}
