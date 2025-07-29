use awsm_renderer_core::{
    renderer::AwsmRendererWebGpu,
    texture::{Extent3d, TextureDescriptor, TextureFormat, TextureUsage},
};

use crate::render::post_process::error::{AwsmPostProcessError, Result};

#[derive(Default)]
pub struct RenderTextures {
    pub formats: RenderTextureFormats,
    ping_pong: bool,
    inner: Option<RenderTexturesInner>,
}

type ViewChanged = bool;

#[derive(Clone, Debug)]
pub struct RenderTextureFormats {
    pub scene: TextureFormat,
    pub accumulation: TextureFormat,
    pub clip_position: TextureFormat,
    pub depth: TextureFormat,
}

impl Default for RenderTextureFormats {
    fn default() -> Self {
        Self {
            scene: TextureFormat::Rgba16float, // HDR format for bloom/tonemapping
            accumulation: TextureFormat::Rgba16float, // HDR format for bloom/tonemapping
            clip_position: TextureFormat::Rgba32float, // High-precision format for position
            depth: TextureFormat::Depth24plus,
        }
    }
}

impl RenderTextures {
    pub fn new(formats: RenderTextureFormats) -> Self {
        Self {
            formats,
            ping_pong: false,
            inner: None,
        }
    }

    pub fn toggle_ping_pong(&mut self) -> bool {
        self.ping_pong = !self.ping_pong;
        self.ping_pong
    }

    pub fn views(&mut self, gpu: &AwsmRendererWebGpu) -> Result<(RenderTextureViews, ViewChanged)> {
        let current_size = gpu.current_context_texture_size()?;
        match self.inner.as_ref() {
            Some(inner) if (inner.width, inner.height) == current_size => {
                return Ok((RenderTextureViews::new(inner, self.ping_pong), false));
                // No change in size, return existing views
            }
            _ => {}
        }

        if let Some(inner) = self.inner.take() {
            inner.destroy();
        }

        let inner =
            RenderTexturesInner::new(gpu, self.formats.clone(), current_size.0, current_size.1)?;
        self.inner = Some(inner);

        Ok((
            RenderTextureViews::new(self.inner.as_ref().unwrap(), self.ping_pong),
            true,
        ))
    }
}

pub struct RenderTextureViews {
    pub scene: web_sys::GpuTextureView,
    pub depths: [web_sys::GpuTextureView; 2],
    pub accumulations: [web_sys::GpuTextureView; 2],
    pub clip_positions: [web_sys::GpuTextureView; 2],
    ping_pong: bool,
}

impl RenderTextureViews {
    pub fn new(inner: &RenderTexturesInner, ping_pong: bool) -> Self {
        Self {
            scene: inner.scene_texture_view.clone(),
            depths: [
                inner.depth_texture_views[0].clone(),
                inner.depth_texture_views[1].clone(),
            ],
            accumulations: [
                inner.accumulation_texture_views[0].clone(),
                inner.accumulation_texture_views[1].clone(),
            ],
            clip_positions: [
                inner.clip_position_texture_views[0].clone(),
                inner.clip_position_texture_views[1].clone(),
            ],
            ping_pong,
        }
    }

    pub fn clip_position_render_target(&self) -> &web_sys::GpuTextureView {
        if !self.ping_pong {
            &self.clip_positions[0]
        } else {
            &self.clip_positions[1]
        }
    }

    pub fn accumulation_render_target(&self) -> &web_sys::GpuTextureView {
        if !self.ping_pong {
            &self.accumulations[1]
        } else {
            &self.accumulations[0]
        }
    }

    pub fn depth_render_target(&self) -> &web_sys::GpuTextureView {
        if !self.ping_pong {
            &self.depths[0]
        } else {
            &self.depths[1]
        }
    }
}

#[allow(dead_code)]
pub struct RenderTexturesInner {
    pub scene_texture: web_sys::GpuTexture,
    pub scene_texture_view: web_sys::GpuTextureView,
    pub depth_textures: [web_sys::GpuTexture; 2],
    pub depth_texture_views: [web_sys::GpuTextureView; 2],
    pub accumulation_textures: [web_sys::GpuTexture; 2],
    pub accumulation_texture_views: [web_sys::GpuTextureView; 2],
    pub clip_position_textures: [web_sys::GpuTexture; 2],
    pub clip_position_texture_views: [web_sys::GpuTextureView; 2],
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
        let scene_texture = gpu.create_texture(
            &TextureDescriptor::new(
                render_texture_formats.scene,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new()
                    .with_render_attachment()
                    .with_texture_binding(),
            )
            .with_label("Scene texture")
            .into(),
        )?;
        let depth_textures = [
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.depth,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new().with_render_attachment(),
                )
                .into(),
            )?,
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.depth,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new().with_render_attachment(),
                )
                .into(),
            )?,
        ];

        let accumulation_textures = [
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.accumulation,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("Scene texture 1")
                .into(),
            )?,
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.accumulation,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("Scene texture 2")
                .into(),
            )?,
        ];

        let clip_position_textures = [
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.clip_position,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("Clip position texture 1")
                .into(),
            )?,
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.clip_position,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label("Clip position texture 2")
                .into(),
            )?,
        ];

        let scene_texture_view = scene_texture
            .create_view()
            .map_err(|e| AwsmPostProcessError::RenderTextureView(format!("scene: {e:?}")))?;
        let depth_texture_views = [
            depth_textures[0]
                .create_view()
                .map_err(|e| AwsmPostProcessError::RenderTextureView(format!("depth: {e:?}")))?,
            depth_textures[1]
                .create_view()
                .map_err(|e| AwsmPostProcessError::RenderTextureView(format!("depth: {e:?}")))?,
        ];

        let accumulation_texture_views = [
            accumulation_textures[0].create_view().map_err(|e| {
                AwsmPostProcessError::RenderTextureView(format!("accumulation: {e:?}"))
            })?,
            accumulation_textures[1].create_view().map_err(|e| {
                AwsmPostProcessError::RenderTextureView(format!("accumulation: {e:?}"))
            })?,
        ];

        let clip_position_texture_views = [
            clip_position_textures[0].create_view().map_err(|e| {
                AwsmPostProcessError::RenderTextureView(format!("clip_position: {e:?}"))
            })?,
            clip_position_textures[1].create_view().map_err(|e| {
                AwsmPostProcessError::RenderTextureView(format!("clip_position: {e:?}"))
            })?,
        ];

        Ok(Self {
            scene_texture,
            scene_texture_view,
            depth_textures,
            depth_texture_views,
            accumulation_textures,
            accumulation_texture_views,
            clip_position_textures,
            clip_position_texture_views,
            width,
            height,
        })
    }

    pub fn destroy(self) {
        self.scene_texture.destroy();
        for texture in self.depth_textures {
            texture.destroy();
        }

        for texture in self.accumulation_textures {
            texture.destroy();
        }
        for texture in self.clip_position_textures {
            texture.destroy();
        }
    }
}
