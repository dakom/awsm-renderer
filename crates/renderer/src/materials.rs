use awsm_renderer_core::{error::AwsmCoreError, renderer::AwsmRendererWebGpu};
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use thiserror::Error;

use crate::{
    bind_groups::{AwsmBindGroupError, BindGroups},
    materials::pbr::{PbrMaterial, PbrMaterials},
    textures::{AwsmTextureError, SamplerKey, TextureKey},
    AwsmRendererLogging,
};

pub mod pbr;

pub struct Materials {
    lookup: SlotMap<MaterialKey, Material>,
    // optimization to avoid loading whole material to check if it has alpha blend
    alpha_blend: SecondaryMap<MaterialKey, bool>,
    pub pbr: PbrMaterials,
}

impl Materials {
    pub fn new(gpu: &AwsmRendererWebGpu) -> Result<Self> {
        Ok(Materials {
            lookup: SlotMap::with_key(),
            alpha_blend: SecondaryMap::new(),
            pbr: PbrMaterials::new(gpu)?,
        })
    }

    pub fn get(&self, key: MaterialKey) -> Option<&Material> {
        self.lookup.get(key)
    }

    pub fn insert(&mut self, material: Material) -> MaterialKey {
        let key = self.lookup.insert(material.clone());
        self.alpha_blend.insert(key, material.has_alpha_blend());
        self.update(key, |_| {});

        key
    }

    pub fn update(&mut self, key: MaterialKey, mut f: impl FnMut(&mut Material)) {
        if let Some(material) = self.lookup.get_mut(key) {
            let old_has_alpha_blend = material.has_alpha_blend();
            f(material);
            let new_has_alpha_blend = material.has_alpha_blend();
            if old_has_alpha_blend != new_has_alpha_blend {
                self.alpha_blend.insert(key, new_has_alpha_blend);
            }
            match material {
                Material::Pbr(pbr_material) => {
                    self.pbr.update(key, pbr_material);
                }
            }
        }
    }

    pub fn has_alpha_blend(&self, key: MaterialKey) -> Result<bool> {
        self.alpha_blend
            .get(key)
            .cloned()
            .ok_or(AwsmMaterialError::MissingAlphaBlendLookup(key))
    }

    pub fn write_gpu(
        &mut self,
        logging: &AwsmRendererLogging,
        gpu: &AwsmRendererWebGpu,
        bind_groups: &mut BindGroups,
    ) -> Result<()> {
        self.pbr.write_gpu(logging, gpu, bind_groups)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Material {
    Pbr(PbrMaterial),
}

impl Material {
    // Needed at top-level for renderer to order correctly
    pub fn has_alpha_blend(&self) -> bool {
        match self {
            Material::Pbr(pbr_material) => pbr_material.has_alpha_blend(),
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
    #[error("[material] missing alpha blend lookup: {0:?}")]
    MissingAlphaBlendLookup(MaterialKey),

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
}
