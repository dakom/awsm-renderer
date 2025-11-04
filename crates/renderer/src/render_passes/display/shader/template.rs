use askama::Template;
use std::collections::HashSet;

use crate::{
    render_passes::{
        display::shader::cache_key::ShaderCacheKeyDisplay,
        geometry::shader::cache_key::ShaderCacheKeyGeometry,
        material::template::{
            ShaderTemplateVertexLocation, ShaderTemplateVertexToFragmentAssignment,
        },
    },
    shaders::{print_shader_source, AwsmShaderError, Result},
};

#[derive(Debug)]
pub struct ShaderTemplateDisplay {
    pub vertex: ShaderTemplateDisplayVertex,
    pub fragment: ShaderTemplateDisplayFragment,
}

#[derive(Template, Debug)]
#[template(path = "display_wgsl/vertex.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateDisplayVertex {
    pub smaa_anti_alias: bool,
}

impl ShaderTemplateDisplayVertex {
    pub fn new(cache_key: &ShaderCacheKeyDisplay) -> Self {
        Self {
            smaa_anti_alias: cache_key.smaa_anti_alias,
        }
    }
}

#[derive(Template, Debug)]
#[template(path = "display_wgsl/fragment.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateDisplayFragment {
    pub smaa_anti_alias: bool,
}

impl ShaderTemplateDisplayFragment {
    pub fn new(cache_key: &ShaderCacheKeyDisplay) -> Self {
        Self {
            smaa_anti_alias: cache_key.smaa_anti_alias,
        }
    }
}

impl TryFrom<&ShaderCacheKeyDisplay> for ShaderTemplateDisplay {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyDisplay) -> Result<Self> {
        Ok(Self {
            vertex: ShaderTemplateDisplayVertex::new(value),
            fragment: ShaderTemplateDisplayFragment::new(value),
        })
    }
}

impl ShaderTemplateDisplay {
    pub fn into_source(self) -> Result<String> {
        let vertex_source = self.vertex.render()?;
        let fragment_source = self.fragment.render()?;
        Ok(format!("{}\n{}", vertex_source, fragment_source))
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Display")
    }
}
