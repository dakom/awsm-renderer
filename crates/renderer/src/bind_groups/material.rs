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

pub struct MaterialBindGroups {
    bind_groups: SecondaryMap<MaterialKey, web_sys::GpuBindGroup>,
    layouts: SlotMap<MaterialBindGroupLayoutKey, web_sys::GpuBindGroupLayout>,
    material_layout_mapping: SecondaryMap<MaterialKey, MaterialBindGroupLayoutKey>,
}
pub enum MaterialBindingLayoutEntry {
    Sampler(SamplerBindingLayout),
    Texture(TextureBindingLayout),
}

pub enum MaterialBindingEntry {
    Sampler(web_sys::GpuSampler),
    Texture(web_sys::GpuTextureView),
}
impl Default for MaterialBindGroups {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialBindGroups {
    pub fn new() -> Self {
        Self {
            bind_groups: SecondaryMap::new(),
            layouts: SlotMap::with_key(),
            material_layout_mapping: SecondaryMap::new(),
        }
    }

    pub fn remove_material(&mut self, key: MaterialKey) -> Result<()> {
        if let Some(layout_key) = self.material_layout_mapping.remove(key) {
            self.layouts.remove(layout_key);
        }
        self.bind_groups.remove(key);
        Ok(())
    }

    pub fn gpu_material_bind_group(&self, key: MaterialKey) -> Result<&web_sys::GpuBindGroup> {
        let bind_group = self
            .bind_groups
            .get(key)
            .ok_or(AwsmBindGroupError::MissingMaterial(key))?;
        Ok(bind_group)
    }

    pub fn gpu_material_bind_group_layout(
        &self,
        key: MaterialKey,
    ) -> Result<&web_sys::GpuBindGroupLayout> {
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

    pub fn insert_layout(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        layout_entries: Vec<MaterialBindingLayoutEntry>,
    ) -> Result<MaterialBindGroupLayoutKey> {
        let entries = layout_entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| match entry {
                MaterialBindingLayoutEntry::Sampler(sampler) => BindGroupLayoutEntry::new(
                    index as u32,
                    BindGroupLayoutResource::Sampler(sampler),
                )
                .with_visibility_fragment(),
                MaterialBindingLayoutEntry::Texture(texture) => BindGroupLayoutEntry::new(
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

    pub fn insert_material(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        material_key: MaterialKey,
        layout_key: MaterialBindGroupLayoutKey,
        entries: &[MaterialBindingEntry],
    ) -> Result<()> {
        let layout = self
            .layouts
            .get(layout_key)
            .ok_or(AwsmBindGroupError::MissingMaterialLayout(layout_key))?;

        let entries = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| match entry {
                MaterialBindingEntry::Sampler(sampler) => {
                    BindGroupEntry::new(index as u32, BindGroupResource::Sampler(sampler))
                }
                MaterialBindingEntry::Texture(texture_view) => {
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
