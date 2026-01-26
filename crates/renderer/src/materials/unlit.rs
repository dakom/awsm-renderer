//! Unlit material parameters.

use crate::{
    materials::{
        writer::{write, Value},
        MaterialAlphaMode, MaterialShaderId, MaterialTexture, Result,
    },
    textures::{SamplerKey, Textures},
};

/// Unlit material parameters.
#[derive(Clone, Debug)]
pub struct UnlitMaterial {
    pub base_color_tex: Option<MaterialTexture>,
    pub base_color_factor: [f32; 4],
    pub emissive_tex: Option<MaterialTexture>,
    pub emissive_factor: [f32; 3],
    // immutable properties, changing them requires recreating the material
    alpha_mode: MaterialAlphaMode,
    double_sided: bool,
}

impl UnlitMaterial {
    /// Creates an unlit material.
    pub fn new(alpha_mode: MaterialAlphaMode, double_sided: bool) -> Self {
        Self {
            base_color_tex: None,
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            emissive_tex: None,
            emissive_factor: [0.0, 0.0, 0.0],
            alpha_mode,
            double_sided,
        }
    }
    /// Returns true if the material should render in the transparency pass.
    pub fn is_transparency_pass(&self) -> bool {
        self.has_alpha_blend() || self.alpha_cutoff().is_some()
    }

    /// Returns the material alpha mode.
    pub fn alpha_mode(&self) -> &MaterialAlphaMode {
        &self.alpha_mode
    }

    /// Returns whether the material is double sided.
    pub fn double_sided(&self) -> bool {
        self.double_sided
    }

    /// Returns the alpha cutoff for masked materials.
    pub fn alpha_cutoff(&self) -> Option<f32> {
        match self.alpha_mode() {
            MaterialAlphaMode::Mask { cutoff } => Some(*cutoff),
            _ => None,
        }
    }

    /// Returns true if alpha blending is enabled.
    pub fn has_alpha_blend(&self) -> bool {
        matches!(self.alpha_mode(), MaterialAlphaMode::Blend)
    }

    /// Returns the alpha mask cutoff, if any.
    pub fn alpha_mask(&self) -> Option<f32> {
        match self.alpha_mode() {
            MaterialAlphaMode::Mask { cutoff } => Some(*cutoff),
            _ => None,
        }
    }

    /// Builds the uniform buffer payload for this material.
    pub fn uniform_buffer_data(&self, textures: &Textures) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(128);

        let sampler_key_list: Vec<SamplerKey> = textures.pool_sampler_set.iter().cloned().collect();
        let map_texture = |tex: &MaterialTexture| {
            crate::materials::writer::map_texture(tex, textures, &sampler_key_list)
        };

        write(&mut data, (MaterialShaderId::Unlit as u32).into());

        write(&mut data, self.alpha_mode().variant_as_u32().into());
        write(&mut data, self.alpha_cutoff().unwrap_or(0.0f32).into());

        if let Some(tex) = self.base_color_tex.as_ref().and_then(map_texture) {
            write(&mut data, tex);
        } else {
            write(&mut data, Value::SkipTexture);
        }
        write(&mut data, self.base_color_factor[0].into());
        write(&mut data, self.base_color_factor[1].into());
        write(&mut data, self.base_color_factor[2].into());
        write(&mut data, self.base_color_factor[3].into());

        if let Some(tex) = self.emissive_tex.as_ref().and_then(map_texture) {
            write(&mut data, tex);
        } else {
            write(&mut data, Value::SkipTexture);
        }
        write(&mut data, self.emissive_factor[0].into());
        write(&mut data, self.emissive_factor[1].into());
        write(&mut data, self.emissive_factor[2].into());

        Ok(data)
    }
}
