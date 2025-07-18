pub mod material_textures;
pub mod uniform_storage;

use awsm_renderer_core::{
    bind_groups::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    },
    error::AwsmCoreError,
    renderer::AwsmRendererWebGpu,
};
use material_textures::{MaterialBindGroupLayoutKey, MaterialTextureBindGroups};
use thiserror::Error;
use uniform_storage::UniformStorageBindGroups;

use crate::{bind_groups::material_textures::MaterialBindGroupKey, materials::MaterialKey};

pub struct BindGroups {
    pub uniform_storages: UniformStorageBindGroups,
    pub material_textures: MaterialTextureBindGroups,
}

impl BindGroups {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        let buffers = UniformStorageBindGroups::new(gpu)?;
        let materials = MaterialTextureBindGroups::new();

        Ok(Self {
            uniform_storages: buffers,
            material_textures: materials,
        })
    }
}

pub(super) fn gpu_create_layout(
    gpu: &AwsmRendererWebGpu,
    label: &'static str,
    entries: Vec<BindGroupLayoutEntry>,
) -> Result<web_sys::GpuBindGroupLayout> {
    gpu.create_bind_group_layout(
        &BindGroupLayoutDescriptor::new(Some(label))
            .with_entries(entries)
            .into(),
    )
    .map_err(|err| AwsmBindGroupError::Layout {
        bind_group: label,
        err,
    })
}

pub(super) fn gpu_create_bind_group(
    gpu: &AwsmRendererWebGpu,
    label: &'static str,
    layout: &web_sys::GpuBindGroupLayout,
    entries: Vec<BindGroupEntry>,
) -> web_sys::GpuBindGroup {
    gpu.create_bind_group(&BindGroupDescriptor::new(layout, Some(label), entries).into())
}

pub(super) type Result<T> = std::result::Result<T, AwsmBindGroupError>;

#[derive(Error, Debug)]
pub enum AwsmBindGroupError {
    #[error("[bind group] Error creating buffer for {label}: {err:?}")]
    CreateBuffer {
        label: &'static str,
        err: AwsmCoreError,
    },
    #[error("[bind group] Error creating bind group layout for group {bind_group}: {err:?}")]
    Layout {
        bind_group: &'static str,
        err: AwsmCoreError,
    },

    #[error("[bind group] Error writing buffer for {label}: {err:?}")]
    WriteBuffer {
        label: &'static str,
        err: AwsmCoreError,
    },

    #[error("[bind group] missing material for {0:?}")]
    MissingMaterialBindGroup(MaterialBindGroupKey),

    #[error("[bind group] missing material layout for {0:?}")]
    MissingMaterialLayout(MaterialBindGroupLayoutKey),

    #[error("[bind group] missing material layout for material {0:?}")]
    MissingMaterialLayoutForMaterial(MaterialKey),

    #[error("[bind group] missing material bind group for material {0:?}")]
    MissingMaterialBindGroupForMaterial(MaterialKey),
}
