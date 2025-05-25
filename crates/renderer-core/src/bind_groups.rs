use crate::{
    buffers::BufferBinding,
    texture::{TextureFormat, TextureSampleType, TextureViewDimension},
};

#[derive(Debug, Clone, Default)]
pub struct BindGroupLayoutDescriptor<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBindGroupLayout#descriptor
    pub label: Option<&'a str>,
    pub entries: Vec<BindGroupLayoutEntry>,
}

impl<'a> BindGroupLayoutDescriptor<'a> {
    pub fn new(label: Option<&'a str>) -> Self {
        Self {
            label,
            entries: Vec::new(),
        }
    }
    pub fn with_push_entry(mut self, entry: BindGroupLayoutEntry) -> Self {
        self.entries.push(entry);
        self
    }

    pub fn with_entries(mut self, entries: Vec<BindGroupLayoutEntry>) -> Self {
        self.entries = entries;
        self
    }
}

#[derive(Debug, Clone)]
pub struct BindGroupLayoutEntry {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBindGroupLayout#entry_objects
    pub binding: u32,
    pub visibility_compute: bool,
    pub visibility_vertex: bool,
    pub visibility_fragment: bool,
    // "Only one may be defined for any given GPUBindGroupLayoutEntry."
    // - https://gpuweb.github.io/gpuweb/#bind-group-layout-creation
    pub resource: BindGroupLayoutResource,
}

impl BindGroupLayoutEntry {
    pub fn new(binding: u32, resource: BindGroupLayoutResource) -> Self {
        Self {
            binding,
            visibility_compute: false,
            visibility_vertex: false,
            visibility_fragment: false,
            resource,
        }
    }

    pub fn with_visibility_compute(mut self) -> Self {
        self.visibility_compute = true;
        self
    }

    pub fn with_visibility_vertex(mut self) -> Self {
        self.visibility_vertex = true;
        self
    }

    pub fn with_visibility_fragment(mut self) -> Self {
        self.visibility_fragment = true;
        self
    }

    pub fn with_visibility_all(mut self) -> Self {
        self.visibility_compute = true;
        self.visibility_vertex = true;
        self.visibility_fragment = true;
        self
    }
}

#[derive(Debug, Clone)]
pub enum BindGroupLayoutResource {
    Buffer(BufferBindingLayout),
    ExternalTexture, // web_sys::GpuExternalTextureBindingLayout::new()
    Sampler(SamplerBindingLayout),
    StorageTexture(StorageTextureBindingLayout),
    Texture(TextureBindingLayout),
}

#[derive(Debug, Clone, Default)]
pub struct BufferBindingLayout {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBindGroupLayout#hasdynamicoffset
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuBufferBindingLayout.html
    pub has_dynamic_offset: Option<bool>,
    pub min_binding_size: Option<usize>,
    pub binding_type: Option<BufferBindingType>,
}

impl BufferBindingLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_dynamic_offset(mut self, has_dynamic_offset: bool) -> Self {
        self.has_dynamic_offset = Some(has_dynamic_offset);
        self
    }

    pub fn with_min_binding_size(mut self, min_binding_size: usize) -> Self {
        self.min_binding_size = Some(min_binding_size);
        self
    }
    pub fn with_binding_type(mut self, binding_type: BufferBindingType) -> Self {
        self.binding_type = Some(binding_type);
        self
    }
}

// https://rustwasm.github.io/wasm-bindgen/api/web_sys/enum.GpuBufferBindingType.html
pub type BufferBindingType = web_sys::GpuBufferBindingType;

#[derive(Debug, Clone, Default)]
pub struct SamplerBindingLayout {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBindGroupLayout#type_2
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuSamplerBindingLayout.html
    pub binding_type: Option<SamplerBindingType>,
}

impl SamplerBindingLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_binding_type(mut self, binding_type: SamplerBindingType) -> Self {
        self.binding_type = Some(binding_type);
        self
    }
}

pub type SamplerBindingType = web_sys::GpuSamplerBindingType;

#[derive(Debug, Clone)]
pub struct StorageTextureBindingLayout {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBindGroupLayout#access
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuStorageTextureBindingLayout.html
    pub access: Option<StorageTextureAccess>,
    pub format: TextureFormat,
    pub view_dimension: Option<TextureViewDimension>,
}

impl StorageTextureBindingLayout {
    pub fn new(format: TextureFormat) -> Self {
        Self {
            format,
            access: None,
            view_dimension: None,
        }
    }

    pub fn with_access(mut self, access: StorageTextureAccess) -> Self {
        self.access = Some(access);
        self
    }
    pub fn with_view_dimension(mut self, view_dimension: TextureViewDimension) -> Self {
        self.view_dimension = Some(view_dimension);
        self
    }
}

// https://rustwasm.github.io/wasm-bindgen/api/web_sys/enum.GpuStorageTextureAccess.html
pub type StorageTextureAccess = web_sys::GpuStorageTextureAccess;

#[derive(Debug, Clone)]
pub struct TextureBindingLayout {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBindGroupLayout#multisampled
    // https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.GpuTextureBindingLayout.html
    pub multisampled: Option<bool>,
    pub view_dimension: Option<TextureViewDimension>,
    pub sample_type: Option<TextureSampleType>,
}

impl Default for TextureBindingLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureBindingLayout {
    pub fn new() -> Self {
        Self {
            multisampled: None,
            view_dimension: None,
            sample_type: None,
        }
    }

    pub fn with_multisampled(mut self, multisampled: bool) -> Self {
        self.multisampled = Some(multisampled);
        self
    }

    pub fn with_view_dimension(mut self, view_dimension: TextureViewDimension) -> Self {
        self.view_dimension = Some(view_dimension);
        self
    }

    pub fn with_sample_type(mut self, sample_type: TextureSampleType) -> Self {
        self.sample_type = Some(sample_type);
        self
    }
}

#[derive(Debug, Clone)]
pub struct BindGroupDescriptor<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBindGroup#descriptor
    pub layout: &'a web_sys::GpuBindGroupLayout,
    pub label: Option<&'a str>,
    pub entries: Vec<BindGroupEntry<'a>>,
}

#[derive(Debug, Clone)]
pub struct BindGroupEntry<'a> {
    // https://developer.mozilla.org/en-US/docs/Web/API/GPUDevice/createBindGroup#entries
    pub binding: u32,
    pub resource: BindGroupResource<'a>,
}

impl<'a> BindGroupEntry<'a> {
    pub fn new(binding: u32, resource: BindGroupResource<'a>) -> Self {
        Self { binding, resource }
    }
}

#[derive(Debug, Clone)]
pub enum BindGroupResource<'a> {
    Buffer(BufferBinding<'a>),
    ExternalTexture(&'a web_sys::GpuExternalTexture),
    Sampler(&'a web_sys::GpuSampler),
    TextureView(&'a web_sys::GpuTextureView),
}

impl<'a> BindGroupDescriptor<'a> {
    pub fn new(
        layout: &'a web_sys::GpuBindGroupLayout,
        label: Option<&'a str>,
        entries: Vec<BindGroupEntry<'a>>,
    ) -> Self {
        Self {
            layout,
            label,
            entries,
        }
    }
}

// js conversions

impl From<BindGroupLayoutDescriptor<'_>> for web_sys::GpuBindGroupLayoutDescriptor {
    fn from(layout: BindGroupLayoutDescriptor) -> Self {
        let entries = js_sys::Array::new();
        for entry in layout.entries {
            entries.push(&web_sys::GpuBindGroupLayoutEntry::from(entry));
        }

        let layout_js = web_sys::GpuBindGroupLayoutDescriptor::new(&entries);

        if let Some(label) = layout.label {
            layout_js.set_label(label);
        }

        layout_js
    }
}

impl From<BindGroupLayoutEntry> for web_sys::GpuBindGroupLayoutEntry {
    fn from(entry: BindGroupLayoutEntry) -> Self {
        let mut visibility = 0;
        if entry.visibility_compute {
            visibility |= web_sys::gpu_shader_stage::COMPUTE;
        }
        if entry.visibility_vertex {
            visibility |= web_sys::gpu_shader_stage::VERTEX;
        }
        if entry.visibility_fragment {
            visibility |= web_sys::gpu_shader_stage::FRAGMENT;
        }

        let entry_js = web_sys::GpuBindGroupLayoutEntry::new(entry.binding, visibility);

        match entry.resource {
            BindGroupLayoutResource::Buffer(buffer) => {
                entry_js.set_buffer(&web_sys::GpuBufferBindingLayout::from(buffer));
            }
            BindGroupLayoutResource::ExternalTexture => {
                entry_js.set_external_texture(&web_sys::GpuExternalTextureBindingLayout::new());
            }
            BindGroupLayoutResource::Sampler(sampler) => {
                entry_js.set_sampler(&web_sys::GpuSamplerBindingLayout::from(sampler));
            }
            BindGroupLayoutResource::StorageTexture(storage_texture) => {
                entry_js.set_storage_texture(&web_sys::GpuStorageTextureBindingLayout::from(
                    storage_texture,
                ));
            }
            BindGroupLayoutResource::Texture(texture) => {
                entry_js.set_texture(&web_sys::GpuTextureBindingLayout::from(texture));
            }
        }

        entry_js
    }
}

impl From<BufferBindingLayout> for web_sys::GpuBufferBindingLayout {
    fn from(layout: BufferBindingLayout) -> Self {
        let layout_js = web_sys::GpuBufferBindingLayout::new();

        if let Some(has_dynamic_offset) = layout.has_dynamic_offset {
            layout_js.set_has_dynamic_offset(has_dynamic_offset);
        }

        if let Some(min_binding_size) = layout.min_binding_size {
            layout_js.set_min_binding_size(min_binding_size as f64);
        }

        if let Some(binding_type) = layout.binding_type {
            layout_js.set_type(binding_type);
        }

        layout_js
    }
}

impl From<SamplerBindingLayout> for web_sys::GpuSamplerBindingLayout {
    fn from(layout: SamplerBindingLayout) -> Self {
        let layout_js = web_sys::GpuSamplerBindingLayout::new();

        if let Some(binding_type) = layout.binding_type {
            layout_js.set_type(binding_type);
        }

        layout_js
    }
}
impl From<StorageTextureBindingLayout> for web_sys::GpuStorageTextureBindingLayout {
    fn from(layout: StorageTextureBindingLayout) -> Self {
        let layout_js = web_sys::GpuStorageTextureBindingLayout::new(layout.format);

        if let Some(access) = layout.access {
            layout_js.set_access(access);
        }

        if let Some(view_dimension) = layout.view_dimension {
            layout_js.set_view_dimension(view_dimension);
        }

        layout_js
    }
}
impl From<TextureBindingLayout> for web_sys::GpuTextureBindingLayout {
    fn from(layout: TextureBindingLayout) -> Self {
        let layout_js = web_sys::GpuTextureBindingLayout::new();

        if let Some(multisampled) = layout.multisampled {
            layout_js.set_multisampled(multisampled);
        }

        if let Some(view_dimension) = layout.view_dimension {
            layout_js.set_view_dimension(view_dimension);
        }

        if let Some(sample_type) = layout.sample_type {
            layout_js.set_sample_type(sample_type);
        }

        layout_js
    }
}

impl From<BindGroupDescriptor<'_>> for web_sys::GpuBindGroupDescriptor {
    fn from(bind_group: BindGroupDescriptor) -> Self {
        let entries = js_sys::Array::new();
        for entry in bind_group.entries {
            entries.push(&web_sys::GpuBindGroupEntry::from(entry));
        }

        let bind_group_js = web_sys::GpuBindGroupDescriptor::new(&entries, bind_group.layout);

        if let Some(label) = bind_group.label {
            bind_group_js.set_label(label);
        }

        bind_group_js
    }
}

impl From<BindGroupEntry<'_>> for web_sys::GpuBindGroupEntry {
    fn from(entry: BindGroupEntry) -> Self {
        web_sys::GpuBindGroupEntry::new(
            entry.binding,
            &match entry.resource {
                BindGroupResource::Buffer(buffer) => web_sys::GpuBufferBinding::from(buffer).into(),
                BindGroupResource::ExternalTexture(external_texture) => external_texture.into(),
                BindGroupResource::Sampler(sampler) => sampler.into(),
                BindGroupResource::TextureView(texture_view) => texture_view.into(),
            },
        )
    }
}
