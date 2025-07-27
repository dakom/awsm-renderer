use awsm_renderer_core::{
    renderer::AwsmRendererWebGpu,
    texture::{Extent3d, TextureDescriptor, TextureFormat, TextureUsage},
};

use crate::render::post_process::error::{AwsmPostProcessError, Result};

#[derive(Default)]
pub struct RenderTextures {
    pub formats: RenderTextureFormats,
    inner: Option<RenderTexturesInner>,
}

type ViewChanged = bool;

#[derive(Clone, Debug)]
pub struct RenderTextureFormats {
    pub scene: TextureFormat,
    pub world_position: TextureFormat,
    pub depth: TextureFormat,
}

impl Default for RenderTextureFormats {
    fn default() -> Self {
        Self {
            scene: TextureFormat::Rgba16float, // HDR format for bloom/tonemapping
            world_position: TextureFormat::Rgba32float, // High-precision format for world position
            depth: TextureFormat::Depth24plus,
        }
    }
}

impl RenderTextures {
    pub fn new(formats: RenderTextureFormats) -> Self {
        Self {
            formats,
            inner: None,
        }
    }

    pub fn views(&mut self, gpu: &AwsmRendererWebGpu) -> Result<(RenderTextureViews, ViewChanged)> {
        self.with_inner(gpu, |inner| Ok(inner.views()))
    }

    fn with_inner<T>(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        f: impl FnOnce(&mut RenderTexturesInner) -> Result<T>,
    ) -> Result<(T, ViewChanged)> {
        let current_size = gpu.current_context_texture_size()?;
        match self.inner.as_mut() {
            Some(inner) if (inner.width, inner.height) == current_size => {
                // early exit, we have a previous size and it hasn't changed
                return f(inner).map(|result| (result, false));
            }
            _ => {}
        }

        if let Some(inner) = self.inner.take() {
            inner.destroy();
        }

        let inner =
            RenderTexturesInner::new(gpu, self.formats.clone(), current_size.0, current_size.1)?;
        self.inner = Some(inner);

        f(self.inner.as_mut().unwrap()).map(|result| (result, true))
    }
}

pub struct RenderTextureViews {
    pub scene: web_sys::GpuTextureView,
    pub world_positions: [web_sys::GpuTextureView; 2], // Used for ping-pong rendering so we get current and previous world positions
    pub depth: web_sys::GpuTextureView,
}

impl RenderTextureViews {
    pub fn world_position_current(&self, ping_pong: bool) -> &web_sys::GpuTextureView {
        if ping_pong {
            &self.world_positions[0]
        } else {
            &self.world_positions[1]
        }
    }
}

#[allow(dead_code)]
struct RenderTexturesInner {
    pub scene_texture: web_sys::GpuTexture,
    pub scene_texture_view: web_sys::GpuTextureView,
    pub depth_texture: web_sys::GpuTexture,
    pub depth_texture_view: web_sys::GpuTextureView,
    pub world_position_textures: [web_sys::GpuTexture; 2], // Used for ping-pong rendering so we get current and previous world positions
    pub world_position_texture_views: [web_sys::GpuTextureView; 2], // Used for ping-pong rendering so we get current and previous world positions
    pub width: u32,
    pub height: u32,
    ping_pong_views: bool,
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

        let scene_texture_view = scene_texture
            .create_view()
            .map_err(|e| AwsmPostProcessError::RenderTextureView(format!("scene: {e:?}")))?;

        let create_world_position_texture = |index: usize| {
            gpu.create_texture(
                &TextureDescriptor::new(
                    render_texture_formats.world_position,
                    Extent3d::new(width, Some(height), Some(1)),
                    TextureUsage::new()
                        .with_render_attachment()
                        .with_texture_binding(),
                )
                .with_label(&format!("World position texture {index}"))
                .into(),
            )
        };

        let world_position_textures: [web_sys::GpuTexture; 2] = [
            create_world_position_texture(1)?,
            create_world_position_texture(2)?,
        ];

        let world_position_texture_views: [web_sys::GpuTextureView; 2] = [
            world_position_textures[0].create_view().map_err(|e| {
                AwsmPostProcessError::RenderTextureView(format!("world position: {e:?}"))
            })?,
            world_position_textures[1].create_view().map_err(|e| {
                AwsmPostProcessError::RenderTextureView(format!("world position: {e:?}"))
            })?,
        ];

        let depth_texture = gpu.create_texture(
            &TextureDescriptor::new(
                render_texture_formats.depth,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new().with_render_attachment(),
            )
            .into(),
        )?;

        let depth_texture_view = depth_texture
            .create_view()
            .map_err(|e| AwsmPostProcessError::RenderTextureView(format!("depth: {e:?}")))?;

        Ok(Self {
            scene_texture,
            scene_texture_view,
            depth_texture,
            depth_texture_view,
            world_position_textures,
            world_position_texture_views,
            width,
            height,
            ping_pong_views: false,
        })
    }

    pub fn views(&mut self) -> RenderTextureViews {
        RenderTextureViews {
            scene: self.scene_texture_view.clone(),
            world_positions: self.world_position_texture_views.clone(),
            depth: self.depth_texture_view.clone(),
        }
    }
    pub fn destroy(self) {
        self.depth_texture.destroy();
    }
}
