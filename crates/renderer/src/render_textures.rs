use awsm_renderer_core::{
    error::AwsmCoreError,
    pipeline::fragment::ColorTargetState,
    renderer::AwsmRendererWebGpu,
    texture::{clear::TextureClearer, Extent3d, TextureDescriptor, TextureFormat, TextureUsage},
};
use thiserror::Error;

pub struct RenderTextures {
    pub formats: RenderTextureFormats,
    frame_count: u32,
    inner: Option<RenderTexturesInner>,
}

#[derive(Clone, Debug)]
pub struct RenderTextureFormats {
    // Output from geometry pass
    pub visiblity_data: TextureFormat,
    pub taa_clip_position: TextureFormat,

    // Output from opaque shading pass
    pub opaque_color: TextureFormat,

    // Output from transparent shading pass
    pub oit_rgb: TextureFormat,
    pub oit_alpha: TextureFormat,

    // Output from composite pass
    pub composite: TextureFormat,

    // output from display pass is whatever current gpu texture format is

    // For depth testing and OIT
    pub depth: TextureFormat,
    // note - output from the composite pass will be whatever the gpu texture format is
}

impl RenderTextureFormats {
    pub async fn new(device: &web_sys::GpuDevice) -> Self {
        let actual_rgba32_format = {
            let res = device.create_texture(
                &TextureDescriptor::new(
                    TextureFormat::Rgba32float,
                    Extent3d::new(1, Some(1), Some(1)),
                    TextureUsage::new().with_render_attachment(),
                )
                .into(),
            );

            match res {
                Ok(tex) => {
                    tex.destroy();
                    TextureFormat::Rgba32float
                }
                Err(_) => TextureFormat::Rgba16float,
            }
        };
        Self {
            visiblity_data: actual_rgba32_format,
            taa_clip_position: TextureFormat::Rgba16float,
            opaque_color: TextureFormat::Rgba16float, // HDR format for bloom/tonemapping
            oit_rgb: TextureFormat::Rgba16float,      // HDR format for bloom/tonemapping
            oit_alpha: TextureFormat::R32float,       // Alpha channel for OIT
            composite: TextureFormat::Rgba8unorm,     // Final composite output format
            depth: TextureFormat::Depth24plus,        // Depth format for depth testing
        }
    }
}

impl RenderTextures {
    pub fn new(formats: RenderTextureFormats) -> Self {
        Self {
            formats,
            frame_count: 0,
            inner: None,
        }
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

    pub fn views(&mut self, gpu: &AwsmRendererWebGpu) -> Result<RenderTextureViews> {
        let current_size = gpu
            .current_context_texture_size()
            .map_err(AwsmRenderTextureError::CurrentScreenSize)?;

        let size_changed = match self.inner.as_ref() {
            Some(inner) => (inner.width, inner.height) != current_size,
            None => true,
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

    pub fn clear_opaque_color(&self, gpu: &AwsmRendererWebGpu) -> Result<()> {
        if let Some(inner) = self.inner.as_ref() {
            inner
                .opaque_color_clearer
                .clear(gpu, &inner.opaque_color)
                .map_err(AwsmRenderTextureError::TextureClearerClear)
        } else {
            Ok(())
        }
    }
}

pub struct RenderTextureViews {
    // Output from geometry pass
    pub visibility_data: web_sys::GpuTextureView,
    pub taa_clip_positions: [web_sys::GpuTextureView; 2],

    // Output from opaque shading pass
    pub opaque_color: web_sys::GpuTextureView,

    // Output from transparent shading pass
    pub oit_rgb: web_sys::GpuTextureView,
    pub oit_alpha: web_sys::GpuTextureView,

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
            taa_clip_positions: inner.taa_clip_position_views.clone(),
            opaque_color: inner.opaque_color_view.clone(),
            oit_rgb: inner.oit_rgb_view.clone(),
            oit_alpha: inner.oit_alpha_view.clone(),
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

    pub taa_clip_positions: [web_sys::GpuTexture; 2],
    pub taa_clip_position_views: [web_sys::GpuTextureView; 2],

    pub opaque_color: web_sys::GpuTexture,
    pub opaque_color_clearer: TextureClearer,
    pub opaque_color_view: web_sys::GpuTextureView,

    pub oit_rgb: web_sys::GpuTexture,
    pub oit_rgb_view: web_sys::GpuTextureView,

    pub oit_alpha: web_sys::GpuTexture,
    pub oit_alpha_view: web_sys::GpuTextureView,

    pub depth: web_sys::GpuTexture,
    pub depth_view: web_sys::GpuTextureView,

    pub composite: web_sys::GpuTexture,
    pub composite_view: web_sys::GpuTextureView,

    pub width: u32,
    pub height: u32,
}

impl RenderTexturesInner {
    pub fn new(
        gpu: &AwsmRendererWebGpu,
        render_texture_formats: RenderTextureFormats,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        // 1. Create all textures
        let visibility_data = gpu
            .create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.visiblity_data,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("Material Offset")
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let taa_clip_positions = [
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.taa_clip_position,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("Screen Position (0)")
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?,
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.taa_clip_position,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("Screen Position (1)")
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?,
        ];

        let opaque_color = gpu
            .create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.opaque_color,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_storage_binding()
                        .with_texture_binding()
                        .with_copy_dst(),
                )
                .with_label("Opaque Color")
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let oit_rgb = gpu
            .create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.oit_rgb,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("OIT RGB")
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let oit_alpha = gpu
            .create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.oit_alpha,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("OIT Alpha")
                .into(),
            )
            .map_err(AwsmRenderTextureError::CreateTexture)?;

        let depth = gpu
            .create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.depth,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new().with_render_attachment(),
                )
                .with_label("Depth")
                .into(),
            )
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

        let taa_clip_position_views = [
            taa_clip_positions[0].create_view().map_err(|e| {
                AwsmRenderTextureError::CreateTextureView(format!("taa_clip_positions[0]: {e:?}"))
            })?,
            taa_clip_positions[1].create_view().map_err(|e| {
                AwsmRenderTextureError::CreateTextureView(format!("taa_clip_positions[1]: {e:?}"))
            })?,
        ];

        let opaque_color_view = opaque_color.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("opaque_color: {e:?}"))
        })?;

        let oit_rgb_view = oit_rgb
            .create_view()
            .map_err(|e| AwsmRenderTextureError::CreateTextureView(format!("oit_rgb: {e:?}")))?;

        let oit_alpha_view = oit_alpha
            .create_view()
            .map_err(|e| AwsmRenderTextureError::CreateTextureView(format!("oit_alpha: {e:?}")))?;

        let depth_view = depth
            .create_view()
            .map_err(|e| AwsmRenderTextureError::CreateTextureView(format!("depth: {e:?}")))?;

        let composite_view = composite
            .create_view()
            .map_err(|e| AwsmRenderTextureError::CreateTextureView(format!("composite: {e:?}")))?;

        Ok(Self {
            visibility_data,
            visibility_data_view,

            taa_clip_positions,
            taa_clip_position_views,

            opaque_color,
            opaque_color_clearer: TextureClearer::new(
                gpu,
                render_texture_formats.opaque_color,
                width,
                height,
            )
            .map_err(AwsmRenderTextureError::CreateTextureClearer)?,
            opaque_color_view,

            oit_rgb,
            oit_rgb_view,

            oit_alpha,
            oit_alpha_view,

            depth,
            depth_view,

            composite,
            composite_view,

            width,
            height,
        })
    }

    pub fn destroy(self) {
        self.visibility_data.destroy();
        for texture in self.taa_clip_positions {
            texture.destroy();
        }
        self.opaque_color.destroy();
        self.oit_rgb.destroy();
        self.oit_alpha.destroy();
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
}
