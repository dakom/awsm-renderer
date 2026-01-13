use askama::Template;

use crate::{
    render_passes::material_transparent::shader::cache_key::ShaderCacheKeyMaterialTransparent,
    shaders::{AwsmShaderError, Result},
};

#[derive(Debug)]
pub struct ShaderTemplateMaterialTransparent {
    pub includes: ShaderTemplateTransparentMaterialIncludes,
    pub bind_groups: ShaderTemplateTransparentMaterialBindGroups,
    pub vertex: ShaderTemplateTransparentMaterialVertex,
    pub fragment: ShaderTemplateTransparentMaterialFragment,
}

#[derive(Template, Debug)]
#[template(
    path = "material_transparent_wgsl/includes.wgsl",
    whitespace = "minimize"
)]
pub struct ShaderTemplateTransparentMaterialIncludes {
    pub max_morph_unroll: u32,
    pub max_skin_unroll: u32,
    pub instancing_transforms: bool,
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
    pub color_sets: Option<u32>,
    pub uv_sets: u32,
    pub debug: ShaderTemplateMaterialTransparentDebug,
}
impl ShaderTemplateTransparentMaterialIncludes {
    pub fn new(cache_key: &ShaderCacheKeyMaterialTransparent) -> Self {
        Self {
            max_morph_unroll: 2,
            max_skin_unroll: 2,
            instancing_transforms: cache_key.instancing_transforms,
            texture_pool_arrays_len: cache_key.texture_pool_arrays_len,
            texture_pool_samplers_len: cache_key.texture_pool_samplers_len,
            color_sets: cache_key.attributes.color_sets,
            uv_sets: cache_key.attributes.uv_sets.unwrap_or_default(),
            debug: ShaderTemplateMaterialTransparentDebug::new(),
        }
    }

    pub fn has_lighting_ibl(&self) -> bool {
        match self.debug.lighting {
            ShaderTemplateMaterialTransparentDebugLighting::None => true,
            ShaderTemplateMaterialTransparentDebugLighting::IblOnly => true,
            ShaderTemplateMaterialTransparentDebugLighting::PunctualOnly => false,
        }
    }

    pub fn has_lighting_punctual(&self) -> bool {
        match self.debug.lighting {
            ShaderTemplateMaterialTransparentDebugLighting::None => true,
            ShaderTemplateMaterialTransparentDebugLighting::IblOnly => false,
            ShaderTemplateMaterialTransparentDebugLighting::PunctualOnly => true,
        }
    }
}

#[derive(Template, Debug)]
#[template(
    path = "material_transparent_wgsl/bind_groups.wgsl",
    whitespace = "minimize"
)]
pub struct ShaderTemplateTransparentMaterialBindGroups {
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
    pub multisampled_geometry: bool,
}

impl ShaderTemplateTransparentMaterialBindGroups {
    pub fn new(cache_key: &ShaderCacheKeyMaterialTransparent) -> Self {
        Self {
            texture_pool_arrays_len: cache_key.texture_pool_arrays_len,
            texture_pool_samplers_len: cache_key.texture_pool_samplers_len,
            multisampled_geometry: cache_key.msaa_sample_count.is_some(),
        }
    }
}

#[derive(Template, Debug)]
#[template(
    path = "material_transparent_wgsl/vertex.wgsl",
    whitespace = "minimize"
)]
pub struct ShaderTemplateTransparentMaterialVertex {
    pub instancing_transforms: bool,
    pub uv_sets: u32,
    pub color_sets: u32,
    pub in_uv_set_start: u32,
    pub in_color_set_start: u32,
    pub out_uv_set_start: u32,
    pub out_color_set_start: u32,
}

impl ShaderTemplateTransparentMaterialVertex {
    pub fn new(cache_key: &ShaderCacheKeyMaterialTransparent) -> Self {
        let uv_sets = cache_key.attributes.uv_sets.unwrap_or_default();
        let color_sets = cache_key.attributes.color_sets.unwrap_or_default();

        // after instancing or tangent
        let in_color_set_start = if cache_key.instancing_transforms {
            7
        } else {
            3
        };

        let in_uv_set_start = in_color_set_start + color_sets;

        let out_color_set_start = 3; // after world_tanget
        let out_uv_set_start = out_color_set_start + color_sets;

        Self {
            instancing_transforms: cache_key.instancing_transforms,
            uv_sets,
            color_sets,
            in_uv_set_start,
            in_color_set_start,
            out_uv_set_start,
            out_color_set_start,
        }
    }
}

#[derive(Template, Debug)]
#[template(
    path = "material_transparent_wgsl/fragment.wgsl",
    whitespace = "minimize"
)]
pub struct ShaderTemplateTransparentMaterialFragment {
    pub uv_sets: u32,
    pub color_sets: u32,
    pub in_uv_set_start: u32,
    pub in_color_set_start: u32,
    pub texture_pool_arrays_len: u32,
    pub texture_pool_samplers_len: u32,
    pub transmission_blur_rings: u32, // more rings = higher quality = more expensive
    pub debug: ShaderTemplateMaterialTransparentDebug,
}

impl ShaderTemplateTransparentMaterialFragment {
    pub fn new(cache_key: &ShaderCacheKeyMaterialTransparent) -> Self {
        let uv_sets = cache_key.attributes.uv_sets.unwrap_or_default();
        let color_sets = cache_key.attributes.color_sets.unwrap_or_default();
        let in_color_set_start = 3; // after world_tangent
        let in_uv_set_start = in_color_set_start + color_sets;

        Self {
            uv_sets,
            color_sets,
            in_uv_set_start,
            in_color_set_start,
            texture_pool_arrays_len: cache_key.texture_pool_arrays_len,
            texture_pool_samplers_len: cache_key.texture_pool_samplers_len,
            transmission_blur_rings: 3,
            debug: ShaderTemplateMaterialTransparentDebug::new(),
        }
    }

    pub fn has_lighting_ibl(&self) -> bool {
        match self.debug.lighting {
            ShaderTemplateMaterialTransparentDebugLighting::None => true,
            ShaderTemplateMaterialTransparentDebugLighting::IblOnly => true,
            ShaderTemplateMaterialTransparentDebugLighting::PunctualOnly => false,
        }
    }

    pub fn has_lighting_punctual(&self) -> bool {
        match self.debug.lighting {
            ShaderTemplateMaterialTransparentDebugLighting::None => true,
            ShaderTemplateMaterialTransparentDebugLighting::IblOnly => false,
            ShaderTemplateMaterialTransparentDebugLighting::PunctualOnly => true,
        }
    }
}

impl TryFrom<&ShaderCacheKeyMaterialTransparent> for ShaderTemplateMaterialTransparent {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyMaterialTransparent) -> Result<Self> {
        Ok(Self {
            includes: ShaderTemplateTransparentMaterialIncludes::new(value),
            bind_groups: ShaderTemplateTransparentMaterialBindGroups::new(value),
            vertex: ShaderTemplateTransparentMaterialVertex::new(value),
            fragment: ShaderTemplateTransparentMaterialFragment::new(value),
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShaderTemplateMaterialTransparentDebug {
    lighting: ShaderTemplateMaterialTransparentDebugLighting,
}

impl ShaderTemplateMaterialTransparentDebug {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn any(&self) -> bool {
        !matches!(
            self.lighting,
            ShaderTemplateMaterialTransparentDebugLighting::None
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ShaderTemplateMaterialTransparentDebugLighting {
    #[default]
    None,
    IblOnly,
    PunctualOnly,
}

impl ShaderTemplateMaterialTransparent {
    pub fn into_source(self) -> Result<String> {
        let includes_source = self.includes.render()?;
        let bind_groups_source = self.bind_groups.render()?;
        let vertex_source = self.vertex.render()?;
        let fragment_source = self.fragment.render()?;

        // print_shader_source(&includes_source, true);

        // debug_unique_string(1, &vertex_source, || {
        //     print_shader_source(&vertex_source, false)
        // });

        Ok(format!(
            "{}\n{}\n{}\n{}",
            includes_source, bind_groups_source, vertex_source, fragment_source
        ))
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Material Transparent")
    }
}
