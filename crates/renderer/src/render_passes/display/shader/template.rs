use askama::Template;

use crate::{
    post_process::ToneMapping,
    render_passes::display::shader::cache_key::ShaderCacheKeyDisplay,
    shaders::{AwsmShaderError, Result},
};

#[derive(Debug)]
pub struct ShaderTemplateDisplay {
    pub bind_groups: ShaderTemplateDisplayBindGroups,
    pub vertex: ShaderTemplateDisplayVertex,
    pub fragment: ShaderTemplateDisplayFragment,
}

#[derive(Template, Debug)]
#[template(path = "display_wgsl/bind_groups.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateDisplayBindGroups {
    pub smaa_anti_alias: bool,
    pub multisampled_geometry: bool,
    pub dof: bool,
    pub debug: ShaderTemplateDisplayDebug,
}

impl ShaderTemplateDisplayBindGroups {
    pub fn new(cache_key: &ShaderCacheKeyDisplay) -> Self {
        Self {
            smaa_anti_alias: cache_key.smaa_anti_alias,
            multisampled_geometry: cache_key.multisampled_geometry,
            dof: cache_key.dof,
            debug: ShaderTemplateDisplayDebug::new(),
        }
    }
}

#[derive(Template, Debug)]
#[template(path = "display_wgsl/vertex.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateDisplayVertex {
    pub smaa_anti_alias: bool,
    pub debug: ShaderTemplateDisplayDebug,
}

impl ShaderTemplateDisplayVertex {
    pub fn new(cache_key: &ShaderCacheKeyDisplay) -> Self {
        Self {
            smaa_anti_alias: cache_key.smaa_anti_alias,
            debug: ShaderTemplateDisplayDebug::new(),
        }
    }
}

#[derive(Template, Debug)]
#[template(path = "display_wgsl/fragment.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateDisplayFragment {
    pub smaa_anti_alias: bool,
    pub tonemapping: ToneMapping,
    pub bloom: bool,
    pub dof: bool,
    pub debug: ShaderTemplateDisplayDebug,
}

impl ShaderTemplateDisplayFragment {
    pub fn new(cache_key: &ShaderCacheKeyDisplay) -> Self {
        Self {
            smaa_anti_alias: cache_key.smaa_anti_alias,
            tonemapping: cache_key.tonemapping,
            bloom: cache_key.bloom,
            dof: cache_key.dof,
            debug: ShaderTemplateDisplayDebug::new(),
        }
    }
}

impl TryFrom<&ShaderCacheKeyDisplay> for ShaderTemplateDisplay {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyDisplay) -> Result<Self> {
        Ok(Self {
            bind_groups: ShaderTemplateDisplayBindGroups::new(value),
            vertex: ShaderTemplateDisplayVertex::new(value),
            fragment: ShaderTemplateDisplayFragment::new(value),
        })
    }
}

impl ShaderTemplateDisplay {
    pub fn into_source(self) -> Result<String> {
        let bind_groups_source = self.bind_groups.render()?;
        let vertex_source = self.vertex.render()?;
        let fragment_source = self.fragment.render()?;
        Ok(format!(
            "{}\n{}\n{}",
            bind_groups_source, vertex_source, fragment_source
        ))
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Display")
    }
}

#[derive(Default, Debug, Clone)]
pub struct ShaderTemplateDisplayDebug {
    pub smaa_edges: bool,
}

impl ShaderTemplateDisplayDebug {
    pub fn new() -> Self {
        Self { smaa_edges: false }
    }
}
