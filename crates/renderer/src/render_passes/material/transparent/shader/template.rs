use askama::Template;

use crate::{
    render_passes::material::transparent::shader::cache_key::ShaderCacheKeyMaterialTransparent,
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
}
impl ShaderTemplateTransparentMaterialIncludes {
    pub fn new(cache_key: &ShaderCacheKeyMaterialTransparent) -> Self {
        Self {
            max_morph_unroll: 2,
            max_skin_unroll: 2,
            instancing_transforms: cache_key.instancing_transforms,
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
}

impl ShaderTemplateTransparentMaterialVertex {
    pub fn new(cache_key: &ShaderCacheKeyMaterialTransparent) -> Self {
        Self {
            instancing_transforms: cache_key.instancing_transforms,
        }
    }
}

#[derive(Template, Debug)]
#[template(
    path = "material_transparent_wgsl/fragment.wgsl",
    whitespace = "minimize"
)]
pub struct ShaderTemplateTransparentMaterialFragment {}

impl ShaderTemplateTransparentMaterialFragment {
    pub fn new(_cache_key: &ShaderCacheKeyMaterialTransparent) -> Self {
        Self {}
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

impl ShaderTemplateMaterialTransparent {
    pub fn into_source(self) -> Result<String> {
        let includes_source = self.includes.render()?;
        let bind_groups_source = self.bind_groups.render()?;
        let vertex_source = self.vertex.render()?;
        let fragment_source = self.fragment.render()?;
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
