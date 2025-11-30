use askama::Template;

use crate::{
    render_passes::composite::shader::cache_key::ShaderCacheKeyComposite,
    shaders::{print_shader_source, AwsmShaderError, Result},
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
pub struct ShaderTemplateCompositeCompute {}

impl ShaderTemplateCompositeCompute {
    pub fn new(cache_key: &ShaderCacheKeyComposite) -> Self {
        Self {}
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

        //print_shader_source(&bind_groups_source, true);

        // debug_unique_string(1, &compute_source, || {
        //     print_shader_source(&vertex_source, false)
        // });

        Ok(format!("{}\n{}", bind_groups_source, compute_source))
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Composite")
    }
}
