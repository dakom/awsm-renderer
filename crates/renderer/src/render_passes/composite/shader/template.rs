use askama::Template;

use crate::{
    render_passes::composite::shader::cache_key::ShaderCacheKeyComposite,
    shaders::{AwsmShaderError, Result},
};

#[derive(Debug)]
pub struct ShaderTemplateComposite {
    pub bind_groups: ShaderTemplateCompositeBindGroups,
    pub compute: ShaderTemplateCompositeCompute,
}

#[derive(Template, Debug)]
#[template(path = "composite_wgsl/bind_groups.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateCompositeBindGroups {
    pub multisampled_geometry: bool,
}

impl ShaderTemplateCompositeBindGroups {
    pub fn new(cache_key: &ShaderCacheKeyComposite) -> Self {
        Self {
            multisampled_geometry: cache_key.multisampled_geometry,
        }
    }
}

#[derive(Template, Debug)]
#[template(path = "composite_wgsl/compute.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateCompositeCompute {
    pub multisampled_geometry: bool,
}

impl ShaderTemplateCompositeCompute {
    pub fn new(cache_key: &ShaderCacheKeyComposite) -> Self {
        Self {
            multisampled_geometry: cache_key.multisampled_geometry,
        }
    }
}

impl TryFrom<&ShaderCacheKeyComposite> for ShaderTemplateComposite {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyComposite) -> Result<Self> {
        Ok(Self {
            bind_groups: ShaderTemplateCompositeBindGroups::new(value),
            compute: ShaderTemplateCompositeCompute::new(value),
        })
    }
}

impl ShaderTemplateComposite {
    pub fn into_source(self) -> Result<String> {
        let bind_groups_source = self.bind_groups.render()?;
        let compute_source = self.compute.render()?;
        Ok(format!("{}\n{}", bind_groups_source, compute_source))
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Composite")
    }
}
