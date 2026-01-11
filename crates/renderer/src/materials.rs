use awsm_renderer_core::{error::AwsmCoreError, renderer::AwsmRendererWebGpu};
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::{AwsmBindGroupError, BindGroups},
    materials::pbr::{PbrMaterial, PbrMaterialBuffers},
    textures::{AwsmTextureError, Textures},
    AwsmRendererLogging,
};

pub mod pbr;

pub struct Materials {
    lookup: SlotMap<MaterialKey, Material>,
    buffers: MaterialBuffers,
    _is_transparency_pass: SecondaryMap<MaterialKey, ()>,
}

struct MaterialBuffers {
    pbr: PbrMaterialBuffers,
    // optimization to avoid loading whole material to find the correct buffer
    buffer_kind: SecondaryMap<MaterialKey, MaterialBufferKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialBufferKind {
    Pbr,
}

impl MaterialBuffers {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(MaterialBuffers {
            pbr: PbrMaterialBuffers::new(gpu)?,
            buffer_kind: SecondaryMap::new(),
        })
    }

    pub fn buffer_offset(&self, key: MaterialKey) -> Result<usize> {
        self.buffer_kind
            .get(key)
            .and_then(|kind| match kind {
                MaterialBufferKind::Pbr => self.pbr.buffer_offset(key),
            })
            .ok_or(AwsmMaterialError::BufferSlotMissing(key))
    }
}

impl Materials {
    pub const MAX_SIZE: usize = 512; // minUniformBufferOffsetAlignment (also, largest possible material size)
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Materials {
            lookup: SlotMap::with_key(),
            buffers: MaterialBuffers::new(gpu)?,
            _is_transparency_pass: SecondaryMap::new(),
        })
    }

    pub fn get(&self, key: MaterialKey) -> Result<&Material> {
        self.lookup.get(key).ok_or(AwsmMaterialError::NotFound(key))
    }

    pub fn insert(&mut self, material: Material, textures: &Textures) -> MaterialKey {
        let is_transparency_pass = material.is_transparency_pass();
        let buffer_kind = material.buffer_kind();

        let key = self.lookup.insert(material);
        if is_transparency_pass {
            self._is_transparency_pass.insert(key, ());
        }
        self.buffers.buffer_kind.insert(key, buffer_kind);
        self.update(key, textures, |_| {});

        key
    }

    pub fn buffer_offset(&self, key: MaterialKey) -> Result<usize> {
        let offset = self.buffers.buffer_offset(key)?;

        #[cfg(debug_assertions)]
        {
            let max: usize = f32::MAX.to_bits() as usize;
            if offset >= max {
                tracing::error!(
                    "[material] material buffer offset {} exceeds f32 max {} - see note in material compute shader",
                    offset, max
                );
            }
        }

        Ok(offset)
    }

    pub fn gpu_buffer(&self, kind: MaterialBufferKind) -> &web_sys::GpuBuffer {
        match kind {
            MaterialBufferKind::Pbr => &self.buffers.pbr.gpu_buffer,
        }
    }

    pub fn update(
        &mut self,
        key: MaterialKey,
        textures: &Textures,
        mut f: impl FnMut(&mut Material),
    ) {
        if let Some(material) = self.lookup.get_mut(key) {
            let old_is_transparency_pass = material.is_transparency_pass();
            let old_buffer_kind = material.buffer_kind();
            f(material);
            let new_is_transparency_pass = material.is_transparency_pass();
            let new_buffer_kind = material.buffer_kind();
            if old_is_transparency_pass != new_is_transparency_pass {
                if new_is_transparency_pass {
                    self._is_transparency_pass.insert(key, ());
                } else {
                    self._is_transparency_pass.remove(key);
                }
            }
            if old_buffer_kind != new_buffer_kind {
                match old_buffer_kind {
                    MaterialBufferKind::Pbr => {
                        self.buffers.pbr.remove(key);
                    }
                }
                self.buffers.buffer_kind.insert(key, new_buffer_kind);
            }
            match material {
                Material::Pbr(pbr_material) => {
                    self.buffers.pbr.update(key, pbr_material, textures);
                }
            }
        }
    }

    pub fn buffer_kind(&self, key: MaterialKey) -> Result<MaterialBufferKind> {
        self.buffers
            .buffer_kind
            .get(key)
            .copied()
            .ok_or(AwsmMaterialError::BufferSlotMissing(key))
    }

    pub fn is_transparency_pass(&self, key: MaterialKey) -> bool {
        self._is_transparency_pass.contains_key(key)
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        self.buffers.pbr.write_gpu(logging, gpu, bind_groups)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Material {
    Pbr(PbrMaterial),
}

impl Material {
    // this should match `mesh_buffer_geometry_kind()`
    pub fn is_transparency_pass(&self) -> bool {
        match self {
            Material::Pbr(pbr_material) => pbr_material.is_transparency_pass(),
        }
    }

    pub fn unlit(&self) -> bool {
        match self {
            Material::Pbr(pbr_material) => pbr_material.unlit(),
        }
    }

    pub fn alpha_mask(&self) -> Option<f32> {
        match self {
            Material::Pbr(pbr_material) => pbr_material.alpha_mask(),
        }
    }

    pub fn buffer_kind(&self) -> MaterialBufferKind {
        match self {
            Material::Pbr(_) => MaterialBufferKind::Pbr,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum MaterialAlphaMode {
    #[default]
    Opaque,
    Mask {
        cutoff: f32,
    },
    Blend,
}

impl MaterialAlphaMode {
    pub fn variant_as_u32(&self) -> u32 {
        match self {
            Self::Opaque => 0,
            Self::Mask { .. } => 1,
            Self::Blend => 2,
        }
    }
}

new_key_type! {
    pub struct MaterialKey;
}

pub type Result<T> = std::result::Result<T, AwsmMaterialError>;

#[derive(Error, Debug)]
pub enum AwsmMaterialError {
    #[error("[material] not found: {0:?}")]
    NotFound(MaterialKey),
    #[error("[material] missing alpha blend lookup: {0:?}")]
    MissingAlphaBlendLookup(MaterialKey),

    #[error("[material] missing alpha cutoff lookup: {0:?}")]
    MissingAlphaCutoffLookup(MaterialKey),

    #[error("[material] create texture view: {0}")]
    CreateTextureView(String),

    #[error("[material] unable to create bind group: {0:?}")]
    MaterialBindGroup(AwsmBindGroupError),

    #[error("[material] unable to create bind group layout: {0:?}")]
    MaterialBindGroupLayout(AwsmBindGroupError),

    #[error("[material] unable to set alpha cutoff, alpha mode is {0:?}")]
    InvalidAlphaModeForCutoff(MaterialAlphaMode),

    #[error("[material] pbr unable to resize bind group: {0:?}")]
    PbrMaterialBindGroupResize(AwsmBindGroupError),

    #[error("[material] pbr unable to write bind group: {0:?}")]
    PbrMaterialBindGroupWrite(AwsmBindGroupError),

    #[error("[material] {0:?}")]
    Core(#[from] AwsmCoreError),

    #[error("[material] {0:?}")]
    Texture(#[from] AwsmTextureError),

    #[error("[material] buffer slot missing {0:?}")]
    BufferSlotMissing(MaterialKey),
}
