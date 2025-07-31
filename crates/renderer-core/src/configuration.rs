use crate::texture::{TextureFormat, TextureUsage};

// device is _not_ included in the configuration, we get that at build time
// https://developer.mozilla.org/en-US/docs/Web/API/GPUCanvasContext/configure#configuration
// https://docs.rs/web-sys/latest/web_sys/struct.GpuCanvasConfiguration.html
#[derive(Default)]
pub struct CanvasConfiguration {
    // if not set, will be derived at build time via get_preferred_canvas_format()
    pub format: Option<TextureFormat>,
    pub alpha_mode: Option<CanvasAlphaMode>,
    pub tone_mapping: Option<CanvasToneMappingMode>,
    pub usage: Option<TextureUsage>,
}

impl CanvasConfiguration {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_format(mut self, format: TextureFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn with_alpha_mode(mut self, alpha_mode: CanvasAlphaMode) -> Self {
        self.alpha_mode = Some(alpha_mode);
        self
    }

    pub fn with_tone_mapping(mut self, tone_mapping: CanvasToneMappingMode) -> Self {
        self.tone_mapping = Some(tone_mapping);
        self
    }
    pub fn with_usage(mut self, usage: TextureUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

// https://docs.rs/web-sys/latest/web_sys/enum.GpuCanvasAlphaMode.html
pub type CanvasAlphaMode = web_sys::GpuCanvasAlphaMode;
// https://docs.rs/web-sys/latest/web_sys/enum.GpuCanvasToneMappingMode.html
pub type CanvasToneMappingMode = web_sys::GpuCanvasToneMappingMode;

impl CanvasConfiguration {
    // https://docs.rs/web-sys/latest/web_sys/struct.GpuCanvasConfiguration.html
    pub fn into_js(
        self,
        gpu: &web_sys::Gpu,
        device: &web_sys::GpuDevice,
    ) -> web_sys::GpuCanvasConfiguration {
        let format = self
            .format
            .unwrap_or_else(|| gpu.get_preferred_canvas_format());

        tracing::info!("Using canvas format: {:?}", format);
        let configuration_js = web_sys::GpuCanvasConfiguration::new(device, format);

        if let Some(alpha_mode) = self.alpha_mode {
            configuration_js.set_alpha_mode(alpha_mode);
        }
        if let Some(usage) = self.usage {
            configuration_js.set_usage(usage.as_u32());
        }
        if let Some(mode) = self.tone_mapping {
            let tone_mapping_js = web_sys::GpuCanvasToneMapping::new();
            tone_mapping_js.set_mode(mode);
            configuration_js.set_tone_mapping(&tone_mapping_js);
        }

        configuration_js
    }
}
