use awsm_renderer_core::{
    renderer::AwsmRendererWebGpu,
    texture::{Extent3d, TextureDescriptor, TextureFormat, TextureUsage},
};

use crate::error::AwsmError;

pub struct RenderTextures {
    pub scene_texture_format: TextureFormat,
    pub depth_texture_format: TextureFormat,
    inner: Option<RenderTexturesInner>,
}

type ViewChanged = bool;

impl RenderTextures {
    pub fn new(scene_texture_format: TextureFormat, depth_texture_format: TextureFormat) -> Self {
        Self {
            scene_texture_format,
            depth_texture_format,
            inner: None,
        }
    }

    pub fn views(
        &mut self,
        gpu: &AwsmRendererWebGpu,
    ) -> crate::error::Result<(RenderTextureViews, ViewChanged)> {
        self.with_inner(gpu, |inner| Ok(inner.views()))
    }

    fn with_inner<T>(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        f: impl FnOnce(&RenderTexturesInner) -> crate::error::Result<T>,
    ) -> crate::error::Result<(T, ViewChanged)> {
        let current_size = gpu.current_context_texture_size()?;
        match self.inner.as_ref() {
            Some(inner) if (inner.width, inner.height) == current_size => {
                // early exit, we have a previous size and it hasn't changed
                return f(inner).map(|result| (result, false));
            }
            _ => {}
        }

        if let Some(inner) = self.inner.take() {
            inner.destroy();
        }

        let inner = RenderTexturesInner::new(
            gpu,
            self.scene_texture_format,
            self.depth_texture_format,
            current_size.0,
            current_size.1,
        )?;
        self.inner = Some(inner);

        f(&self.inner.as_ref().unwrap()).map(|result| (result, true))
    }
}

pub struct RenderTextureViews {
    pub scene: web_sys::GpuTextureView,
    pub depth: web_sys::GpuTextureView,
}

struct RenderTexturesInner {
    pub scene_texture: web_sys::GpuTexture,
    pub scene_texture_view: web_sys::GpuTextureView,
    pub depth_texture: web_sys::GpuTexture,
    pub depth_texture_view: web_sys::GpuTextureView,
    pub width: u32,
    pub height: u32,
}

impl RenderTexturesInner {
    pub fn new(
        gpu: &AwsmRendererWebGpu,
        scene_texture_format: TextureFormat,
        depth_texture_format: TextureFormat,
        width: u32,
        height: u32,
    ) -> crate::error::Result<Self> {
        let scene_texture = gpu.create_texture(
            &TextureDescriptor::new(
                scene_texture_format,
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
            .map_err(|e| AwsmError::SceneTextureCreateView(e.as_string().unwrap_or_default()))?;

        let depth_texture = gpu.create_texture(
            &TextureDescriptor::new(
                depth_texture_format,
                Extent3d::new(width, Some(height), Some(1)),
                TextureUsage::new().with_render_attachment(),
            )
            .into(),
        )?;

        let depth_texture_view = depth_texture
            .create_view()
            .map_err(|e| AwsmError::DepthTextureCreateView(e.as_string().unwrap_or_default()))?;

        Ok(Self {
            scene_texture,
            scene_texture_view,
            depth_texture,
            depth_texture_view,
            width,
            height,
        })
    }

    pub fn views(&self) -> RenderTextureViews {
        RenderTextureViews {
            scene: self.scene_texture_view.clone(),
            depth: self.depth_texture_view.clone(),
        }
    }
    pub fn destroy(self) {
        self.depth_texture.destroy();
    }
}
