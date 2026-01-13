use crate::{
    materials::{
        writer::{write, Value},
        MaterialAlphaMode, MaterialShaderId, MaterialTexture, Result,
    },
    textures::{SamplerKey, Textures},
};

#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub base_color_tex: Option<MaterialTexture>,
    pub base_color_factor: [f32; 4],

    pub metallic_roughness_tex: Option<MaterialTexture>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,

    pub normal_tex: Option<MaterialTexture>,
    pub normal_scale: f32,

    pub occlusion_tex: Option<MaterialTexture>,
    pub occlusion_strength: f32,

    pub emissive_tex: Option<MaterialTexture>,
    pub emissive_factor: [f32; 3],

    // Debug settings
    pub debug: PbrMaterialDebug,

    // Non-core features and extensions
    pub vertex_color_info: Option<PbrMaterialVertexColorInfo>,
    pub emissive_strength: Option<PbrMaterialEmissiveStrength>,
    pub ior: Option<PbrMaterialIor>,
    pub specular: Option<PbrMaterialSpecular>,
    pub transmission: Option<PbrMaterialTransmission>,
    pub diffuse_transmission: Option<PbrMaterialDiffuseTransmission>,
    pub volume: Option<PbrMaterialVolume>,
    pub clearcoat: Option<PbrMaterialClearCoat>,
    pub sheen: Option<PbrMaterialSheen>,
    pub dispersion: Option<PbrMaterialDispersion>,
    pub anisotropy: Option<PbrMaterialAnisotropy>,
    pub iridescence: Option<PbrMaterialIridescence>,
    // things that affect shader generation and therefore can't be changed dynamically (create a new material instead)
    //
    alpha_mode: MaterialAlphaMode,
    double_sided: bool,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum PbrMaterialDebug {
    None,
    BaseColor,
    MetallicRoughness,
    Normals,
    Occlusion,
    Emissive,
    Specular,
}

impl PbrMaterialDebug {
    pub fn bitmask(&self) -> u32 {
        match self {
            PbrMaterialDebug::None => 0,
            PbrMaterialDebug::BaseColor => 1 << 0,
            PbrMaterialDebug::MetallicRoughness => 1 << 1,
            PbrMaterialDebug::Normals => 1 << 2,
            PbrMaterialDebug::Occlusion => 1 << 3,
            PbrMaterialDebug::Emissive => 1 << 4,
            PbrMaterialDebug::Specular => 1 << 5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PbrMaterialVertexColorInfo {
    pub set_index: u32,
}

#[derive(Clone, Debug)]
pub struct PbrMaterialEmissiveStrength {
    pub strength: f32,
}

#[derive(Clone, Debug)]
pub struct PbrMaterialIor {
    pub ior: f32,
}

#[derive(Clone, Debug)]
pub struct PbrMaterialSpecular {
    pub tex: Option<MaterialTexture>,
    pub factor: f32,
    pub color_tex: Option<MaterialTexture>,
    pub color_factor: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct PbrMaterialTransmission {
    pub tex: Option<MaterialTexture>,
    pub factor: f32,
}

#[derive(Clone, Debug)]
pub struct PbrMaterialDiffuseTransmission {
    pub tex: Option<MaterialTexture>,
    pub factor: f32,
    pub color_tex: Option<MaterialTexture>,
    pub color_factor: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct PbrMaterialVolume {
    // volume extension
    pub thickness_tex: Option<MaterialTexture>,
    pub thickness_factor: f32,
    pub attenuation_distance: f32,
    pub attenuation_color: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct PbrMaterialClearCoat {
    // clearcoat extension
    pub tex: Option<MaterialTexture>,
    pub factor: f32,
    pub roughness_tex: Option<MaterialTexture>,
    pub roughness_factor: f32,
    pub normal_tex: Option<MaterialTexture>,
    pub normal_scale: f32,
}

#[derive(Clone, Debug)]
pub struct PbrMaterialSheen {
    pub roughness_tex: Option<MaterialTexture>,
    pub roughness_factor: f32,
    pub color_tex: Option<MaterialTexture>,
    pub color_factor: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct PbrMaterialDispersion {
    pub dispersion: f32,
}

#[derive(Clone, Debug)]
pub struct PbrMaterialAnisotropy {
    pub tex: Option<MaterialTexture>,
    pub strength: f32,
    pub rotation: f32,
}

#[derive(Clone, Debug)]
pub struct PbrMaterialIridescence {
    pub tex: Option<MaterialTexture>,
    pub factor: f32,
    pub ior: f32,
    pub thickness_tex: Option<MaterialTexture>,
    pub thickness_min: f32,
    pub thickness_max: f32,
}

impl PbrMaterial {
    pub fn new(alpha_mode: MaterialAlphaMode, double_sided: bool) -> Self {
        Self {
            base_color_tex: None,
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic_roughness_tex: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            normal_tex: None,
            normal_scale: 1.0,
            occlusion_tex: None,
            occlusion_strength: 1.0,
            emissive_tex: None,
            emissive_factor: [0.0, 0.0, 0.0],
            vertex_color_info: None,
            emissive_strength: None,
            ior: None,
            specular: None,
            transmission: None,
            diffuse_transmission: None,
            volume: None,
            clearcoat: None,
            sheen: None,
            dispersion: None,
            anisotropy: None,
            iridescence: None,
            debug: PbrMaterialDebug::None,
            alpha_mode,
            double_sided,
        }
    }

    // this should match `mesh_buffer_geometry_kind()`
    pub fn is_transparency_pass(&self) -> bool {
        self.has_alpha_blend() || self.alpha_cutoff().is_some() || self.has_transmission()
    }

    /// Returns true if the material has any transmission effect
    /// (either via transmission_factor > 0 or a transmission texture)
    pub fn has_transmission(&self) -> bool {
        match &self.transmission {
            Some(transmission) => transmission.factor > 0.0 || transmission.tex.is_some(),
            None => false,
        }
    }

    pub fn alpha_mode(&self) -> &MaterialAlphaMode {
        &self.alpha_mode
    }

    pub fn double_sided(&self) -> bool {
        self.double_sided
    }

    pub fn alpha_cutoff(&self) -> Option<f32> {
        match self.alpha_mode() {
            MaterialAlphaMode::Mask { cutoff } => Some(*cutoff),
            _ => None,
        }
    }

    pub fn has_alpha_blend(&self) -> bool {
        matches!(self.alpha_mode(), MaterialAlphaMode::Blend)
    }

    pub fn alpha_mask(&self) -> Option<f32> {
        match self.alpha_mode() {
            MaterialAlphaMode::Mask { cutoff } => Some(*cutoff),
            _ => None,
        }
    }

    pub fn uniform_buffer_data(&self, textures: &Textures) -> Result<Vec<u8>> {
        let mut data: Vec<u8> = Vec::with_capacity(256);

        let sampler_key_list: Vec<SamplerKey> = textures.pool_sampler_set.iter().cloned().collect();
        let map_texture = |tex: &MaterialTexture| {
            crate::materials::writer::map_texture(tex, textures, &sampler_key_list)
        };

        write(&mut data, (MaterialShaderId::Pbr as u32).into());

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

        if let Some(tex) = self.metallic_roughness_tex.as_ref().and_then(map_texture) {
            write(&mut data, tex);
        } else {
            write(&mut data, Value::SkipTexture);
        }
        write(&mut data, self.metallic_factor.into());
        write(&mut data, self.roughness_factor.into());

        if let Some(tex) = self.normal_tex.as_ref().and_then(map_texture) {
            write(&mut data, tex);
        } else {
            write(&mut data, Value::SkipTexture);
        }
        write(&mut data, self.normal_scale.into());

        if let Some(tex) = self.occlusion_tex.as_ref().and_then(map_texture) {
            write(&mut data, tex);
        } else {
            write(&mut data, Value::SkipTexture);
        }
        write(&mut data, self.occlusion_strength.into());

        if let Some(tex) = self.emissive_tex.as_ref().and_then(map_texture) {
            write(&mut data, tex);
        } else {
            write(&mut data, Value::SkipTexture);
        }
        write(&mut data, self.emissive_factor[0].into());
        write(&mut data, self.emissive_factor[1].into());
        write(&mut data, self.emissive_factor[2].into());

        write(&mut data, self.debug.bitmask().into());

        // feature indices,
        #[derive(Default, Debug)]
        struct FeatureIndices {
            pub vertex_color_info: u32,
            pub emissive_strength: u32,
            pub ior: u32,
            pub specular: u32,
            pub transmission: u32,
            pub diffuse_transmission: u32,
            pub volume: u32,
            pub clearcoat: u32,
            pub sheen: u32,
            pub dispersion: u32,
            pub anisotropy: u32,
            pub iridescence: u32,
        }

        impl FeatureIndices {
            pub fn to_u32_array(&self) -> [u32; 12] {
                [
                    self.vertex_color_info,
                    self.emissive_strength,
                    self.ior,
                    self.specular,
                    self.transmission,
                    self.diffuse_transmission,
                    self.volume,
                    self.clearcoat,
                    self.sheen,
                    self.dispersion,
                    self.anisotropy,
                    self.iridescence,
                ]
            }
        }
        // first write feature_indices as a placeholder, then we go back and fill them in
        let mut feature_indices = FeatureIndices::default();
        let indices_offset = data.len();
        for value in feature_indices.to_u32_array() {
            data.extend_from_slice(&value.to_le_bytes());
        }

        // features...
        fn current_index(data: &[u8]) -> u32 {
            let index = data.len() as u32 / 4;
            // subtract by 1 for the shader id
            index - 1
        }

        if let Some(PbrMaterialVertexColorInfo { set_index }) = self.vertex_color_info {
            feature_indices.vertex_color_info = current_index(&data);
            write(&mut data, set_index.into());
        }

        if let Some(PbrMaterialEmissiveStrength { strength }) = self.emissive_strength {
            feature_indices.emissive_strength = current_index(&data);
            write(&mut data, strength.into());
        }

        if let Some(PbrMaterialIor { ior }) = self.ior {
            feature_indices.ior = current_index(&data);
            write(&mut data, ior.into());
        }

        if let Some(PbrMaterialSpecular {
            tex,
            factor,
            color_tex,
            color_factor,
        }) = &self.specular
        {
            feature_indices.specular = current_index(&data);

            if let Some(tex) = tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }

            write(&mut data, factor.into());

            if let Some(tex) = color_tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }
            write(&mut data, color_factor[0].into());
            write(&mut data, color_factor[1].into());
            write(&mut data, color_factor[2].into());
        }

        if let Some(PbrMaterialTransmission { tex, factor }) = &self.transmission {
            feature_indices.transmission = current_index(&data);

            if let Some(tex) = tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }

            write(&mut data, factor.into());
        }

        if let Some(PbrMaterialDiffuseTransmission {
            tex,
            factor,
            color_tex,
            color_factor,
        }) = &self.diffuse_transmission
        {
            feature_indices.diffuse_transmission = current_index(&data);

            if let Some(tex) = tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }

            write(&mut data, factor.into());

            if let Some(tex) = color_tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }
            write(&mut data, color_factor[0].into());
            write(&mut data, color_factor[1].into());
            write(&mut data, color_factor[2].into());
        }

        if let Some(PbrMaterialVolume {
            thickness_tex,
            thickness_factor,
            attenuation_distance,
            attenuation_color,
        }) = &self.volume
        {
            feature_indices.volume = current_index(&data);

            if let Some(tex) = thickness_tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }

            write(&mut data, thickness_factor.into());
            write(&mut data, attenuation_distance.into());
            write(&mut data, attenuation_color[0].into());
            write(&mut data, attenuation_color[1].into());
            write(&mut data, attenuation_color[2].into());
        }

        if let Some(PbrMaterialClearCoat {
            tex,
            factor,
            roughness_tex,
            roughness_factor,
            normal_tex,
            normal_scale,
        }) = &self.clearcoat
        {
            feature_indices.clearcoat = current_index(&data);

            if let Some(tex) = tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }

            write(&mut data, factor.into());

            if let Some(tex) = roughness_tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }
            write(&mut data, roughness_factor.into());

            if let Some(tex) = normal_tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }
            write(&mut data, normal_scale.into());
        }

        if let Some(PbrMaterialSheen {
            roughness_tex,
            roughness_factor,
            color_tex,
            color_factor,
        }) = &self.sheen
        {
            feature_indices.sheen = current_index(&data);

            if let Some(tex) = roughness_tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }
            write(&mut data, roughness_factor.into());

            if let Some(tex) = color_tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }
            write(&mut data, color_factor[0].into());
            write(&mut data, color_factor[1].into());
            write(&mut data, color_factor[2].into());
        }

        if let Some(PbrMaterialDispersion { dispersion }) = self.dispersion {
            feature_indices.dispersion = current_index(&data);
            write(&mut data, dispersion.into());
        }

        if let Some(PbrMaterialAnisotropy {
            tex,
            strength,
            rotation,
        }) = &self.anisotropy
        {
            feature_indices.anisotropy = current_index(&data);

            if let Some(tex) = tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }

            write(&mut data, strength.into());
            write(&mut data, rotation.into());
        }

        if let Some(PbrMaterialIridescence {
            tex,
            factor,
            ior,
            thickness_tex,
            thickness_min,
            thickness_max,
        }) = &self.iridescence
        {
            feature_indices.iridescence = current_index(&data);

            if let Some(tex) = tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }

            write(&mut data, factor.into());
            write(&mut data, ior.into());

            if let Some(tex) = thickness_tex.as_ref().and_then(map_texture) {
                write(&mut data, tex);
            } else {
                write(&mut data, Value::SkipTexture);
            }

            write(&mut data, thickness_min.into());
            write(&mut data, thickness_max.into());
        }

        // re-write indices
        for (index, value) in feature_indices.to_u32_array().iter().enumerate() {
            let start_offset = indices_offset + index * 4;
            let end_offset = start_offset + 4;
            let feature_indices_bytes = &mut data[start_offset..end_offset];
            feature_indices_bytes.copy_from_slice(&value.to_le_bytes());
        }

        Ok(data)
    }
}
