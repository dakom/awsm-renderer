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
    layouts: SlotMap<MaterialBindGroupLayoutKey, web_sys::GpuBindGroupLayout>,
    bind_groups: SlotMap<MaterialBindGroupKey, web_sys::GpuBindGroup>,
    // optimizations so we don't have to load the whole material to get the keys
    material_layout_mapping: SecondaryMap<MaterialKey, MaterialBindGroupLayoutKey>,
    material_bind_group_mapping: SecondaryMap<MaterialKey, MaterialBindGroupKey>,
}

#[derive(Debug, Clone)]
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
            layouts: SlotMap::with_key(),
            bind_groups: SlotMap::with_key(),
            material_layout_mapping: SecondaryMap::new(),
            material_bind_group_mapping: SecondaryMap::new(),
        }
    }

    pub fn gpu_bind_group(&self, key: MaterialBindGroupKey) -> Result<&web_sys::GpuBindGroup> {
        let bind_group = self
            .bind_groups
            .get(key)
            .ok_or(AwsmBindGroupError::MissingMaterialBindGroup(key))?;
        Ok(bind_group)
    }

    pub fn gpu_bind_group_by_material(&self, key: MaterialKey) -> Result<&web_sys::GpuBindGroup> {
        let key = self.get_bind_group_key(key)?;
        self.gpu_bind_group(key)
    }

    pub fn gpu_bind_group_layout(
        &self,
        layout_key: MaterialBindGroupLayoutKey,
    ) -> Result<&web_sys::GpuBindGroupLayout> {
        let layout = self
            .layouts
            .get(layout_key)
            .ok_or(AwsmBindGroupError::MissingMaterialLayout(layout_key))?;
        Ok(layout)
    }

    pub fn gpu_bind_group_layout_by_material(
        &self,
        key: MaterialKey,
    ) -> Result<&web_sys::GpuBindGroupLayout> {
        let layout_key = self.get_layout_key(key)?;
        self.gpu_bind_group_layout(layout_key)
    }

    pub fn get_layout_key(&self, key: MaterialKey) -> Result<MaterialBindGroupLayoutKey> {
        let layout_key = *self
            .material_layout_mapping
            .get(key)
            .ok_or(AwsmBindGroupError::MissingMaterialLayoutForMaterial(key))?;
        Ok(layout_key)
    }

    pub fn get_bind_group_key(&self, key: MaterialKey) -> Result<MaterialBindGroupKey> {
        let bind_group_key = *self
            .material_bind_group_mapping
            .get(key)
            .ok_or(AwsmBindGroupError::MissingMaterialBindGroupForMaterial(key))?;
        Ok(bind_group_key)
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

    pub fn insert_bind_group(
        &mut self,
        gpu: &AwsmRendererWebGpu,
        layout_key: MaterialBindGroupLayoutKey,
        entries: &[MaterialTextureBindingEntry],
    ) -> Result<MaterialBindGroupKey> {
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

        let key = self.bind_groups.insert(bind_group);

        Ok(key)
    }

    pub fn insert_material_bind_group_layout_lookup(
        &mut self,
        material_key: MaterialKey,
        bind_group_layout_key: MaterialBindGroupLayoutKey,
    ) {
        self.material_layout_mapping
            .insert(material_key, bind_group_layout_key);
    }

    pub fn insert_material_bind_group_lookup(
        &mut self,
        material_key: MaterialKey,
        bind_group_key: MaterialBindGroupKey,
    ) {
        self.material_bind_group_mapping
            .insert(material_key, bind_group_key);
    }
}

new_key_type! {
    pub struct MaterialBindGroupLayoutKey;
}

new_key_type! {
    pub struct MaterialBindGroupKey;
}
