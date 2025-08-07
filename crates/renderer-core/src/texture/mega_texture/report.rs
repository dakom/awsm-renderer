use crate::texture::mega_texture::MegaTexture;

impl<ID> MegaTexture<ID> {
    pub fn size_report(&self, limits: &web_sys::GpuSupportedLimits) -> MegaTextureSizeReport {
        self.size(limits).into_report()
    }

    fn size(&self, limits: &web_sys::GpuSupportedLimits) -> MegaTextureSize {
        let inner_len: Vec<Vec<usize>> = self
            .atlases
            .iter()
            .map(|atlas| {
                atlas
                    .layers
                    .iter()
                    .map(|layer| layer.entries.len())
                    .collect()
            })
            .collect();

        let inner_size: Vec<Vec<Vec<MegaTextureWidthHeight>>> = self
            .atlases
            .iter()
            .map(|atlas| {
                atlas
                    .layers
                    .iter()
                    .map(|layer| {
                        layer
                            .entries
                            .iter()
                            .map(|entry| entry.image_data.size().into())
                            .collect()
                    })
                    .collect()
            })
            .collect();

        let max_bindings_per_group = limits.max_sampled_textures_per_shader_stage();
        let max_bind_groups = limits.max_bind_groups();

        let max_size_per_bind_group = (
            self.texture_size * self.atlas_depth * max_bindings_per_group,
            self.texture_size * self.atlas_depth * max_bindings_per_group,
        );

        let max_size = (
            max_size_per_bind_group.0 * max_bind_groups,
            max_size_per_bind_group.1 * max_bind_groups,
        );

        MegaTextureSize {
            inner_len,
            texture_size: self.texture_size,
            max_depth: self.atlas_depth,
            inner_size,
            max_size: max_size.into(),
            max_size_per_bind_group: max_size_per_bind_group.into(),
        }
    }
}

#[derive(Clone)]
pub struct MegaTextureSize {
    inner_len: Vec<Vec<usize>>,
    inner_size: Vec<Vec<Vec<MegaTextureWidthHeight>>>,
    texture_size: u32,
    max_depth: u32,
    max_size: MegaTextureWidthHeight,
    max_size_per_bind_group: MegaTextureWidthHeight,
}

impl MegaTextureSize {
    pub fn into_report(self) -> MegaTextureSizeReport {
        let atlas_len = self.inner_len.len();
        let layer_per_atlas_len: Vec<usize> = self.inner_len.iter().map(|l| l.len()).collect();
        let entry_per_layer_per_atlas_len: Vec<Vec<usize>> =
            self.inner_len.iter().map(|l| l.to_vec()).collect();
        let total_entries_len: usize = self
            .inner_len
            .iter()
            .map(|l| l.iter().copied().sum::<usize>())
            .sum();

        let total_layers_len: usize = self.inner_len.iter().map(|l| l.len()).sum();

        let mut entries_size_per_layer_per_atlas: Vec<Vec<Vec<MegaTextureWidthHeight>>> =
            Vec::new();

        for layer in &self.inner_size {
            let mut out_l = Vec::new();
            for entries in layer {
                let mut out_e = Vec::new();
                for entry in entries {
                    out_e.push((*entry));
                }
                out_l.push(out_e);
            }
            entries_size_per_layer_per_atlas.push(out_l);
        }

        let layer_per_atlas_size: Vec<Vec<MegaTextureWidthHeight>> =
            entries_size_per_layer_per_atlas
                .iter()
                .map(|l| {
                    l.iter()
                        .map(|e| {
                            e.iter().fold(MegaTextureWidthHeight::ZERO, |acc, &size| {
                                MegaTextureWidthHeight::new(
                                    acc.width + size.width,
                                    acc.height + size.height,
                                )
                            })
                        })
                        .collect()
                })
                .collect();

        let atlas_sizes: Vec<MegaTextureWidthHeight> = layer_per_atlas_size
            .iter()
            .map(|l| {
                l.iter().fold(MegaTextureWidthHeight::ZERO, |acc, &size| {
                    MegaTextureWidthHeight::new(acc.width + size.width, acc.height + size.height)
                })
            })
            .collect();

        let total_used_size: MegaTextureWidthHeight =
            atlas_sizes
                .iter()
                .fold(MegaTextureWidthHeight::ZERO, |acc, &size| {
                    MegaTextureWidthHeight::new(acc.width + size.width, acc.height + size.height)
                });

        let total_area = MegaTextureSizeReportArea::new(self.max_size, total_used_size);

        let max_atlas_size: MegaTextureWidthHeight = (
            self.max_depth * self.texture_size,
            self.max_depth * self.texture_size,
        )
            .into();
        let atlas_areas = atlas_sizes
            .iter()
            .map(|&size| MegaTextureSizeReportArea::new(max_atlas_size, size))
            .collect::<Vec<_>>();

        let max_layer_size: MegaTextureWidthHeight = (self.texture_size, self.texture_size).into();
        let layer_per_atlas_area: Vec<Vec<MegaTextureSizeReportArea>> = layer_per_atlas_size
            .iter()
            .map(|l| {
                l.iter()
                    .map(|&size| MegaTextureSizeReportArea::new(max_layer_size, size))
                    .collect()
            })
            .collect();

        MegaTextureSizeReport {
            atlas_len,
            layer_per_atlas_len,
            entry_per_layer_per_atlas_len,
            total_entries_len,
            total_layers_len,
            entries_size_per_layer_per_atlas,
            atlas_areas,
            layer_per_atlas_area,
            total_area,
            max_size_per_bind_group: self.max_size_per_bind_group,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MegaTextureWidthHeight {
    pub width: u32,
    pub height: u32,
}

impl MegaTextureWidthHeight {
    pub const ZERO: MegaTextureWidthHeight = MegaTextureWidthHeight {
        width: 0,
        height: 0,
    };
    pub fn new(width: u32, height: u32) -> Self {
        MegaTextureWidthHeight { width, height }
    }
}

impl From<(u32, u32)> for MegaTextureWidthHeight {
    fn from(size: (u32, u32)) -> Self {
        MegaTextureWidthHeight {
            width: size.0,
            height: size.1,
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MegaTextureSizeReport {
    pub atlas_len: usize,
    pub layer_per_atlas_len: Vec<usize>,
    pub entry_per_layer_per_atlas_len: Vec<Vec<usize>>,
    pub total_entries_len: usize,
    pub total_layers_len: usize,
    pub entries_size_per_layer_per_atlas: Vec<Vec<Vec<MegaTextureWidthHeight>>>,
    pub total_area: MegaTextureSizeReportArea,
    pub layer_per_atlas_area: Vec<Vec<MegaTextureSizeReportArea>>,
    pub atlas_areas: Vec<MegaTextureSizeReportArea>,
    pub max_size_per_bind_group: MegaTextureWidthHeight,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MegaTextureSizeReportArea {
    pub perc_free: f64,
    pub perc_used: f64,
    pub max_size: MegaTextureWidthHeight,
    pub max_area: f64,
    pub used_size: MegaTextureWidthHeight,
    pub used_area: f64,
}

impl MegaTextureSizeReportArea {
    pub fn new(max_size: MegaTextureWidthHeight, used_size: MegaTextureWidthHeight) -> Self {
        let max_area = max_size.width as f64 * max_size.height as f64;
        let used_area = used_size.width as f64 * used_size.height as f64;
        let perc_used = (used_area / max_area) * 100.0;
        let perc_free = (1.0 - perc_used / 100.0) * 100.0;

        Self {
            perc_free,
            perc_used,
            max_size,
            max_area,
            used_size,
            used_area,
        }
    }
}

#[cfg(feature = "serde")]
impl MegaTextureSizeReport {
    pub fn console_log(&self) {
        let js_value = serde_wasm_bindgen::to_value(self).unwrap();
        web_sys::console::log_1(&js_value);
    }
}

#[cfg(feature = "serde")]
impl MegaTextureSizeReportArea {
    pub fn console_log(&self) {
        let js_value = serde_wasm_bindgen::to_value(self).unwrap();
        web_sys::console::log_1(&js_value);
    }
}
