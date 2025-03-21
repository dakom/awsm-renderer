use wasm_bindgen::prelude::*;

pub type TextureFormat = web_sys::GpuTextureFormat;
pub type TextureAspect = web_sys::GpuTextureAspect;
pub type TextureViewDimension = web_sys::GpuTextureViewDimension;
pub type TextureSampleType = web_sys::GpuTextureSampleType;
pub type TextureDimension = web_sys::GpuTextureDimension;

#[derive(Debug, Clone)]
pub struct TextureDescriptor<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createTexture#descriptor
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuTextureDescriptor.html
    pub dimension: Option<TextureDimension>,
    pub format: TextureFormat,
    pub label: Option<&'a str>,
    pub mip_level_count: Option<u32>,
    pub sample_count: Option<u32>,
    pub size: TextureDescriptorSize,
    pub usage: TextureDescriptorUsage,
    pub view_formats: Vec<TextureFormat>,
}

impl <'a> TextureDescriptor<'a> {
    pub fn new(format: TextureFormat, size: TextureDescriptorSize, usage: TextureDescriptorUsage) -> Self {
        Self {
            dimension: None,
            format,
            label: None,
            mip_level_count: None,
            sample_count: None,
            size,
            usage,
            view_formats: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TextureDescriptorUsage {
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/gpu_buffer_usage/index.html
    pub copy_dst: bool,
    pub copy_src: bool,
    pub index: bool,
    pub indirect: bool,
    pub map_read: bool,
    pub map_write: bool,
    pub query_resolve: bool,
    pub storage: bool,
    pub uniform: bool,
    pub vertex: bool,
}

impl TextureDescriptorUsage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_u32(&self) -> u32 {
        let mut usage = 0;
        if self.copy_dst { usage |= web_sys::gpu_buffer_usage::COPY_DST; }
        if self.copy_src { usage |= web_sys::gpu_buffer_usage::COPY_SRC; }
        if self.index { usage |= web_sys::gpu_buffer_usage::INDEX; }
        if self.indirect { usage |= web_sys::gpu_buffer_usage::INDIRECT; }
        if self.map_read { usage |= web_sys::gpu_buffer_usage::MAP_READ; }
        if self.map_write { usage |= web_sys::gpu_buffer_usage::MAP_WRITE; }
        if self.query_resolve { usage |= web_sys::gpu_buffer_usage::QUERY_RESOLVE; }
        if self.storage { usage |= web_sys::gpu_buffer_usage::STORAGE; }
        if self.uniform { usage |= web_sys::gpu_buffer_usage::UNIFORM; }
        if self.vertex { usage |= web_sys::gpu_buffer_usage::VERTEX; }
        usage
    }

    pub fn with_copy_dst(mut self) -> Self {
        self.copy_dst = true;
        self
    }

    pub fn with_copy_src(mut self) -> Self {
        self.copy_src = true;
        self
    }

    pub fn with_index(mut self) -> Self {
        self.index = true;
        self
    }

    pub fn with_indirect(mut self) -> Self {
        self.indirect = true;
        self
    }

    pub fn with_map_read(mut self) -> Self {
        self.map_read = true;
        self
    }

    pub fn with_map_write(mut self) -> Self {
        self.map_write = true;
        self
    }

    pub fn with_query_resolve(mut self) -> Self {
        self.query_resolve = true;
        self
    }

    pub fn with_storage(mut self) -> Self {
        self.storage = true;
        self
    }

    pub fn with_uniform(mut self) -> Self {
        self.uniform = true;
        self
    }

    pub fn with_vertex(mut self) -> Self {
        self.vertex = true;
        self
    }
}


#[derive(Debug, Clone)]
pub struct TextureDescriptorSize {
    pub width: u32,
    pub height: Option<u32>,
    pub depth_or_array_layers: Option<u32>,
}

impl TextureDescriptorSize {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createTexture#size

    pub fn new(width: u32, height: Option<u32>, depth_or_array_layers: Option<u32>) -> Self {
        Self { width, height, depth_or_array_layers }
    }

    pub fn with_height(mut self, height: u32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn with_depth_or_array_layers(mut self, depth_or_array_layers: u32) -> Self {
        self.depth_or_array_layers = Some(depth_or_array_layers);
        self
    }
}


#[derive(Debug, Clone, Default)]
pub struct TextureViewDescriptor<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUTexture/createView#descriptor
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuTextureViewDescriptor.html
    pub array_layer_count: Option<u32>,
    pub aspect: Option<TextureAspect>,
    pub base_array_layer: Option<u32>,
    pub base_mip_level: Option<u32>,
    pub dimension: Option<TextureViewDimension>,
    pub format: Option<TextureFormat>,
    pub label: Option<&'a str>,
    pub mip_level_count: Option<u32>,
    pub usage: Option<TextureDescriptorUsage>,
}

impl <'a> TextureViewDescriptor<'a> {
    pub fn new(label: Option<&'a str>) -> Self {
        Self { label, ..Default::default() }
    }
}

pub struct ExternalTextureDescriptor<'a> {
    pub source: ExternalTextureDescriptorSource,
    pub label: Option<&'a str>,
}

impl <'a> ExternalTextureDescriptor<'a> {
    pub fn new(source: ExternalTextureDescriptorSource, label: Option<&'a str>) -> Self {
        Self { source, label }
    }
}

pub enum ExternalTextureDescriptorSource {
    VideoElement(web_sys::HtmlVideoElement),
    VideoFrame(web_sys::VideoFrame),
}

// js conversions

impl From<TextureDescriptor<'_>> for web_sys::GpuTextureDescriptor {
    fn from(descriptor: TextureDescriptor) -> Self {
        let descriptor_js = web_sys::GpuTextureDescriptor::new(descriptor.format, &js_sys::Object::from(descriptor.size), descriptor.usage.as_u32());

        if let Some(dimension) = descriptor.dimension {
            descriptor_js.set_dimension(dimension);
        }

        if let Some(label) = descriptor.label {
            descriptor_js.set_label(label);
        }

        if let Some(mip_level_count) = descriptor.mip_level_count {
            descriptor_js.set_mip_level_count(mip_level_count);
        }
        if let Some(sample_count) = descriptor.sample_count {
            descriptor_js.set_sample_count(sample_count);
        }
        if !descriptor.view_formats.is_empty() {
            let view_formats = js_sys::Array::new();
            for format in descriptor.view_formats {
                view_formats.push(&format.into());
            }
            descriptor_js.set_view_formats(&view_formats);
        }

        descriptor_js
    }
}

impl From<TextureViewDescriptor<'_>> for web_sys::GpuTextureViewDescriptor {
    fn from(descriptor: TextureViewDescriptor) -> Self {
        let descriptor_js = web_sys::GpuTextureViewDescriptor::new();

        if let Some(array_layer_count) = descriptor.array_layer_count {
            descriptor_js.set_array_layer_count(array_layer_count);
        }
        if let Some(aspect) = descriptor.aspect {
            descriptor_js.set_aspect(aspect);
        }
        if let Some(base_array_layer) = descriptor.base_array_layer {
            descriptor_js.set_base_array_layer(base_array_layer);
        }
        if let Some(base_mip_level) = descriptor.base_mip_level {
            descriptor_js.set_base_mip_level(base_mip_level);
        }
        if let Some(dimension) = descriptor.dimension {
            descriptor_js.set_dimension(dimension);
        }
        if let Some(format) = descriptor.format {
            descriptor_js.set_format(format);
        }
        if let Some(label) = descriptor.label {
            descriptor_js.set_label(label);
        }
        if let Some(mip_level_count) = descriptor.mip_level_count {
            descriptor_js.set_mip_level_count(mip_level_count);
        }
        if let Some(usage) = descriptor.usage {
            descriptor_js.set_usage(usage.as_u32());
        }

        descriptor_js
    }
}

impl From<ExternalTextureDescriptor<'_>> for web_sys::GpuExternalTextureDescriptor {
    fn from(descriptor: ExternalTextureDescriptor) -> Self {
        let descriptor_js = web_sys::GpuExternalTextureDescriptor::new(&match descriptor.source {
            ExternalTextureDescriptorSource::VideoElement(video) => video.into(),
            ExternalTextureDescriptorSource::VideoFrame(frame) => frame.into(),
        });

        if let Some(label) = descriptor.label {
            descriptor_js.set_label(label);
        }

        descriptor_js
    }
}

impl From<TextureDescriptorSize> for js_sys::Object {
    fn from(size: TextureDescriptorSize) -> Self {
        // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createTexture#size
        let obj = js_sys::Object::new();

        js_sys::Reflect::set(&obj, &JsValue::from_str("width"), &JsValue::from_f64(size.width as f64)).unwrap();

        if let Some(height) = size.height {
            js_sys::Reflect::set(&obj, &JsValue::from_str("height"), &JsValue::from_f64(height as f64)).unwrap();
        }

        if let Some(depth_or_array_layers) = size.depth_or_array_layers {
            js_sys::Reflect::set(&obj, &JsValue::from_str("depthOrArrayLayers"), &JsValue::from_f64(depth_or_array_layers as f64)).unwrap();
        }

        obj 

    }
}