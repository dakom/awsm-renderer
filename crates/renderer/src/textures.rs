use slotmap::{new_key_type, SlotMap};

pub struct Textures {
    textures: SlotMap<TextureKey, web_sys::GpuTexture>,
    samplers: SlotMap<SamplerKey, web_sys::GpuSampler>,
}

impl Default for Textures {
    fn default() -> Self {
        Self::new()
    }
}

impl Textures {
    pub fn new() -> Self {
        Self {
            textures: SlotMap::with_key(),
            samplers: SlotMap::with_key(),
        }
    }

    pub fn add_texture(&mut self, texture: web_sys::GpuTexture) -> TextureKey {
        self.textures.insert(texture)
    }

    pub fn get_texture(&self, key: TextureKey) -> Option<&web_sys::GpuTexture> {
        self.textures.get(key)
    }

    pub fn remove_texture(&mut self, key: TextureKey) {
        if let Some(texture) = self.textures.remove(key) {
            texture.destroy();
        }
    }

    pub fn add_sampler(&mut self, sampler: web_sys::GpuSampler) -> SamplerKey {
        self.samplers.insert(sampler)
    }

    pub fn get_sampler(&self, key: SamplerKey) -> Option<&web_sys::GpuSampler> {
        self.samplers.get(key)
    }

    pub fn remove_sampler(&mut self, key: SamplerKey) {
        self.samplers.remove(key);
    }
}

new_key_type! {
    pub struct TextureKey;
}

new_key_type! {
    pub struct SamplerKey;
}
