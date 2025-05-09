use crate::materials::MaterialKey;

use super::{gpu_create_bind_group, gpu_create_layout, AwsmBindGroupError, Result};
use awsm_renderer_core::{
    bind_groups::{
        BindGroupEntry, BindGroupLayoutEntry, BindGroupLayoutResource, BindGroupResource,
        SamplerBindingLayout, TextureBindingLayout,
    },
    renderer::AwsmRendererWebGpu,
};
use slotmap::{new_key_type, SecondaryMap, SlotMap};

pub struct MaterialTextureBindGroups {
    bind_groups: SecondaryMap<MaterialKey, web_sys::GpuBindGroup>,
    layouts: SlotMap<MaterialBindGroupLayoutKey, web_sys::GpuBindGroupLayout>,
    material_layout_mapping: SecondaryMap<MaterialKey, MaterialBindGroupLayoutKey>,
}
pub enum MaterialTextureBindingLayoutEntry {
    Sampler(SamplerBindingLayout),
    Texture(TextureBindingLayout),
}

pub enum MaterialTextureBindingEntry {
    Sampler(web_sys::GpuSampler),
    Texture(web_sys::GpuTextureView),
}
impl Default for MaterialTextureBindGroups {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialTextureBindGroups {
    pub fn new() -> Self {
        Self {
            bind_groups: SecondaryMap::new(),
            layouts: SlotMap::with_key(),
            material_layout_mapping: SecondaryMap::new(),
        }
    }

    pub fn remove(&mut self, key: MaterialKey) -> Result<()> {
        if let Some(layout_key) = self.material_layout_mapping.remove(key) {
            self.layouts.remove(layout_key);
        }
        self.bind_groups.remove(key);
        Ok(())
    }

    pub fn gpu_bind_group(&self, key: MaterialKey) -> Result<&web_sys::GpuBindGroup> {
        let bind_group = self
            .bind_groups
            .get(key)
            .ok_or(AwsmBindGroupError::MissingMaterial(key))?;
        Ok(bind_group)
    }

    pub fn gpu_bind_group_layout(&self, key: MaterialKey) -> Result<&web_sys::GpuBindGroupLayout> {
        let layout_key = *self
            .material_layout_mapping
            .get(key)
            .ok_or(AwsmBindGroupError::MissingMaterialLayoutForMaterial(key))?;
        let layout = self
            .layouts
            .get(layout_key)
            .ok_or(AwsmBindGroupError::MissingMaterialLayout(layout_key))?;
        Ok(layout)
    }

    pub fn insert_bind_group_layout(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        layout_entries: Vec<MaterialTextureBindingLayoutEntry>,
    ) -> Result<MaterialBindGroupLayoutKey> {
        let entries = layout_entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| match entry {
                MaterialTextureBindingLayoutEntry::Sampler(sampler) => BindGroupLayoutEntry::new(
                    index as u32,
                    BindGroupLayoutResource::Sampler(sampler),
                )
                .with_visibility_fragment(),
                MaterialTextureBindingLayoutEntry::Texture(texture) => BindGroupLayoutEntry::new(
                    index as u32,
                    BindGroupLayoutResource::Texture(texture),
                )
                .with_visibility_fragment(),
            })
            .collect::<Vec<_>>();

        let layout = gpu_create_layout(gpu, "Material", entries)?;

        let key = self.layouts.insert(layout);

        Ok(key)
    }

    pub fn insert_material_texture(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        material_key: MaterialKey,
        layout_key: MaterialBindGroupLayoutKey,
        entries: &[MaterialTextureBindingEntry],
    ) -> Result<()> {
        let layout = self
            .layouts
            .get(layout_key)
            .ok_or(AwsmBindGroupError::MissingMaterialLayout(layout_key))?;

        let entries = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| match entry {
                MaterialTextureBindingEntry::Sampler(sampler) => {
                    BindGroupEntry::new(index as u32, BindGroupResource::Sampler(sampler))
                }
                MaterialTextureBindingEntry::Texture(texture_view) => {
                    BindGroupEntry::new(index as u32, BindGroupResource::TextureView(texture_view))
                }
            })
            .collect::<Vec<_>>();

        let bind_group = gpu_create_bind_group(gpu, "Material", layout, entries);

        self.bind_groups.insert(material_key, bind_group);
        self.material_layout_mapping
            .insert(material_key, layout_key);

        Ok(())
    }
}

new_key_type! {
    pub struct MaterialBindGroupLayoutKey;
}
