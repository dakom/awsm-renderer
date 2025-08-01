use awsm_renderer_core::{
    error::AwsmCoreError, pipeline::fragment::ColorTargetState, renderer::AwsmRendererWebGpu, texture::{Extent3d, TextureDescriptor, TextureFormat, TextureUsage}
};
use thiserror::Error;

#[derive(Default)]
pub struct RenderTextures {
    pub formats: RenderTextureFormats,
    frame_count: u32,
    inner: Option<RenderTexturesInner>,
}

#[derive(Clone, Debug)]
pub struct RenderTextureFormats {
    // Output from geometry pass
    pub entity_id: TextureFormat,
    pub world_normal: TextureFormat,
    pub screen_pos: TextureFormat,
    pub motion_vector: TextureFormat,

    // Output from opaque shading pass
    pub opaque_color: TextureFormat,

    // Output from transparent shading pass
    pub oit_rgb: TextureFormat,
    pub oit_alpha: TextureFormat,

    // For depth testing and OIT
    pub depth: TextureFormat,

    // note - output from the composite pass will be whatever the gpu texture format is
}

impl Default for RenderTextureFormats {
    fn default() -> Self {
        Self {
            entity_id: TextureFormat::R32uint,
            world_normal: TextureFormat::Rgba16float,
            screen_pos: TextureFormat::Rgba16float, // just xy, z is for depth 
            motion_vector: TextureFormat::Rg32float, // just xy, z is not needed
            opaque_color: TextureFormat::Rgba16float, // HDR format for bloom/tonemapping
            oit_rgb: TextureFormat::Rgba16float, // HDR format for bloom/tonemapping
            oit_alpha: TextureFormat::R16float, // Alpha channel for OIT
            depth: TextureFormat::Depth24plus, // Depth format for depth testing
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
        let current_size = gpu.current_context_texture_size().map_err(AwsmRenderTextureError::CurrentScreenSize)?;
        let current_display = gpu.current_context_texture_view().map_err(AwsmRenderTextureError::CurrentTextureView)?;

        let size_changed = match self.inner.as_ref() {
            Some(inner) => {
                (inner.width, inner.height) != current_size
            },
            None => true
        };

        if size_changed {
            if let Some(inner) = self.inner.take() {
                inner.destroy();
            }

            let inner =
                RenderTexturesInner::new(gpu, self.formats.clone(), current_size.0, current_size.1)?;
            self.inner = Some(inner);
        }

        Ok(RenderTextureViews::new(self.inner.as_ref().unwrap(), current_display, self.ping_pong(), current_size.0, current_size.1, size_changed))
    }
}

pub struct RenderTextureViews {
    pub entity_id: web_sys::GpuTextureView,
    pub world_normal: web_sys::GpuTextureView,
    pub curr_screen_pos: web_sys::GpuTextureView, 
    pub prev_screen_pos: web_sys::GpuTextureView,
    pub motion_vector: web_sys::GpuTextureView,
    pub opaque_color: web_sys::GpuTextureView,
    pub oit_rgb: web_sys::GpuTextureView,
    pub oit_alpha: web_sys::GpuTextureView,
    pub depth: web_sys::GpuTextureView,
    pub composite: web_sys::GpuTextureView,
    pub display: web_sys::GpuTextureView,
    pub size_changed: bool,
    pub width: u32,
    pub height: u32,
}

impl RenderTextureViews {
    pub fn new(inner: &RenderTexturesInner, display: web_sys::GpuTextureView, ping_pong: bool, width: u32, height: u32, size_changed: bool) -> Self {
        let curr_index = if ping_pong { 0 } else { 1 };
        let prev_index = if ping_pong { 1 } else { 0 };
        Self {
            entity_id: inner.entity_id_view.clone(),
            world_normal: inner.world_normal_view.clone(),
            curr_screen_pos: inner.screen_pos_views[curr_index].clone(),
            prev_screen_pos: inner.screen_pos_views[prev_index].clone(),
            motion_vector: inner.motion_vector_view.clone(),
            opaque_color: inner.opaque_color_view.clone(),
            oit_rgb: inner.oit_rgb_view.clone(),
            oit_alpha: inner.oit_alpha_view.clone(),
            depth: inner.depth_view.clone(),
            composite: inner.composite_view.clone(),
            display,
            size_changed,
            width,
            height,
        }
    }
}

#[allow(dead_code)]
pub struct RenderTexturesInner {
    pub entity_id: web_sys::GpuTexture,
    pub entity_id_view: web_sys::GpuTextureView,

    pub world_normal: web_sys::GpuTexture,
    pub world_normal_view: web_sys::GpuTextureView,

    pub screen_pos: [web_sys::GpuTexture;2],
    pub screen_pos_views: [web_sys::GpuTextureView;2],

    pub motion_vector: web_sys::GpuTexture,
    pub motion_vector_view: web_sys::GpuTextureView,

    pub opaque_color: web_sys::GpuTexture,
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
        let entity_id = gpu.create_texture(
            &TextureDescriptor::new(
                render_texture_formats.entity_id,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new()
                    .with_render_attachment()
                    .with_texture_binding(),
            )
            .with_label("Entity Id")
            .into(),
        ).map_err(AwsmRenderTextureError::CreateTexture)?;

        let world_normal = gpu.create_texture(
            &TextureDescriptor::new(
                render_texture_formats.world_normal,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new()
                    .with_render_attachment()
                    .with_texture_binding(),
            )
            .with_label("World Normal")
            .into(),
        ).map_err(AwsmRenderTextureError::CreateTexture)?;

        let screen_pos = [
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.screen_pos,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("Screen Position (0)")
                .into(),
            ).map_err(AwsmRenderTextureError::CreateTexture)?,
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.screen_pos,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("Screen Position (1)")
                .into(),
            ).map_err(AwsmRenderTextureError::CreateTexture)?,
        ];

        let motion_vector = gpu.create_texture(
            &TextureDescriptor::new(
                render_texture_formats.motion_vector,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new()
                    .with_render_attachment()
                    .with_texture_binding(),
            )
            .with_label("Motion Vector")
            .into(),
        ).map_err(AwsmRenderTextureError::CreateTexture)?;

        let opaque_color = gpu.create_texture(
            &TextureDescriptor::new(
                render_texture_formats.opaque_color,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new()
                    .with_render_attachment()
                    .with_texture_binding(),
            )
            .with_label("Opaque Color")
            .into(),
        ).map_err(AwsmRenderTextureError::CreateTexture)?;

        let oit_rgb = gpu.create_texture(
            &TextureDescriptor::new(
                render_texture_formats.oit_rgb,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new()
                    .with_render_attachment()
                    .with_texture_binding(),
            )
            .with_label("OIT RGB")
            .into(),
        ).map_err(AwsmRenderTextureError::CreateTexture)?;

        let oit_alpha = gpu.create_texture(
            &TextureDescriptor::new(
                render_texture_formats.oit_alpha,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new()
                    .with_render_attachment()
                    .with_texture_binding(),
            )
            .with_label("OIT Alpha")
            .into(),
        ).map_err(AwsmRenderTextureError::CreateTexture)?;

        let depth = gpu.create_texture(
            &TextureDescriptor::new(
                render_texture_formats.depth,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new()
                    .with_render_attachment()
                    .with_texture_binding(),
            )
            .with_label("Depth")
            .into(),
        ).map_err(AwsmRenderTextureError::CreateTexture)?;

        let composite = gpu.create_texture(
            &TextureDescriptor::new(
                gpu.current_context_format(),
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new()
                    .with_render_attachment()
                    .with_texture_binding(),
            )
            .with_label("Composite")
            .into(),
        ).map_err(AwsmRenderTextureError::CreateTexture)?;

        // 2. Create views for all textures

        let entity_id_view = entity_id.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("entity_id: {e:?}"))
        })?;

        let world_normal_view = world_normal.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("world_normal: {e:?}"))
        })?;

        let screen_pos_views = [
            screen_pos[0].create_view().map_err(|e| {
                AwsmRenderTextureError::CreateTextureView(format!("screen_pos[0]: {e:?}"))
            })?,
            screen_pos[1].create_view().map_err(|e| {
                AwsmRenderTextureError::CreateTextureView(format!("screen_pos[1]: {e:?}"))
            })?,
        ];

        let motion_vector_view = motion_vector.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("motion_vector: {e:?}"))
        })?;

        let opaque_color_view = opaque_color.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("opaque_color: {e:?}"))
        })?;

        let oit_rgb_view = oit_rgb.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("oit_rgb: {e:?}"))
        })?;

        let oit_alpha_view = oit_alpha.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("oit_alpha: {e:?}"))
        })?;

        let depth_view = depth.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("depth: {e:?}"))
        })?;

        let composite_view = composite.create_view().map_err(|e| {
            AwsmRenderTextureError::CreateTextureView(format!("composite: {e:?}"))
        })?;

        Ok(Self {
            entity_id,
            entity_id_view,

            world_normal,
            world_normal_view,

            screen_pos,
            screen_pos_views,

            motion_vector,
            motion_vector_view,

            opaque_color,
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
        self.entity_id.destroy();
        self.world_normal.destroy();
        for texture in self.screen_pos{
            texture.destroy();
        }
        self.motion_vector.destroy();
        self.opaque_color.destroy();
        self.oit_rgb.destroy();
        self.oit_alpha.destroy();
        self.depth.destroy();
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
}