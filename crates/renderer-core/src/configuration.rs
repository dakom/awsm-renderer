use crate::texture::{TextureFormat, TextureUsage};

// https://developer.mozilla.org/en-US/docs/Web/API/GPUCanvasContext/configure#configuration
pub struct CanvasConfiguration<'a> {
    pub device: &'a web_sys::GpuDevice,
    pub format: TextureFormat,
    pub alpha_mode: Option<CanvasAlphaMode>,
    pub tone_mapping: Option<CanvasToneMapping>,
    pub usage: Option<TextureUsage>,
}

impl<'a> CanvasConfiguration<'a> {
    pub fn new(device: &'a web_sys::GpuDevice, format: TextureFormat) -> Self {
        Self {
            alpha_mode: None,
            device,
            format,
            tone_mapping: None,
            usage: None,
        }
    }

    pub fn with_alpha_mode(mut self, alpha_mode: CanvasAlphaMode) -> Self {
        self.alpha_mode = Some(alpha_mode);
        self
    }

    pub fn with_tone_mapping(mut self, tone_mapping: CanvasToneMapping) -> Self {
        self.tone_mapping = Some(tone_mapping);
        self
    }
    pub fn with_usage(mut self, usage: TextureUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

#[derive(Clone, Default)]
pub struct CanvasToneMapping {
    pub mode: Option<CanvasToneMappingMode>,
}
impl CanvasToneMapping {
    pub fn new() -> Self {
        Self { mode: None }
    }

    pub fn with_mode(mut self, mode: CanvasToneMappingMode) -> Self {
        self.mode = Some(mode);
        self
    }
}

pub type CanvasAlphaMode = web_sys::GpuCanvasAlphaMode;
pub type CanvasToneMappingMode = web_sys::GpuCanvasToneMappingMode;

impl From<CanvasConfiguration<'_>> for web_sys::GpuCanvasConfiguration {
    fn from(config: CanvasConfiguration) -> Self {
        let configuration_js = web_sys::GpuCanvasConfiguration::new(config.device, config.format);

        if let Some(alpha_mode) = config.alpha_mode {
            configuration_js.set_alpha_mode(alpha_mode);
        }
        if let Some(usage) = config.usage {
            configuration_js.set_usage(usage.as_u32());
        }
        if let Some(tone_mapping) = config.tone_mapping {
            configuration_js.set_tone_mapping(&tone_mapping.into());
        }

        configuration_js
    }
}

impl From<CanvasToneMapping> for web_sys::GpuCanvasToneMapping {
    fn from(tone_mapping: CanvasToneMapping) -> Self {
        let tone_mapping_js = web_sys::GpuCanvasToneMapping::new();

        if let Some(mode) = tone_mapping.mode {
            tone_mapping_js.set_mode(mode);
        }

        tone_mapping_js
    }
}
