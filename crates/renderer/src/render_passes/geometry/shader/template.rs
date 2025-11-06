use std::collections::HashSet;

use askama::Template;

use crate::{
    render_passes::{
        geometry::shader::cache_key::ShaderCacheKeyGeometry,
        material::template::{
            ShaderTemplateVertexLocation, ShaderTemplateVertexToFragmentAssignment,
        },
    },
    shaders::{print_shader_source, AwsmShaderError, Result},
};

#[derive(Debug)]
pub struct ShaderTemplateGeometry {
    pub vertex: ShaderTemplateGeometryVertex,
    pub fragment: ShaderTemplateGeometryFragment,
}

#[derive(Template, Debug)]
#[template(path = "geometry_wgsl/vertex.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateGeometryVertex {
    max_morph_unroll: u32,
    max_skin_unroll: u32,
    instancing_transforms: bool,
}

impl ShaderTemplateGeometryVertex {
    pub fn new(cache_key: &ShaderCacheKeyGeometry) -> Self {
        Self {
            max_morph_unroll: 2,
            max_skin_unroll: 2,
            instancing_transforms: cache_key.instancing_transforms,
        }
    }
}

#[derive(Template, Debug)]
#[template(path = "geometry_wgsl/fragment.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateGeometryFragment {}

impl ShaderTemplateGeometryFragment {
    pub fn new(cache_key: &ShaderCacheKeyGeometry) -> Self {
        Self {}
    }
}

impl TryFrom<&ShaderCacheKeyGeometry> for ShaderTemplateGeometry {
    type Error = AwsmShaderError;

    fn try_from(value: &ShaderCacheKeyGeometry) -> Result<Self> {
        Ok(Self {
            vertex: ShaderTemplateGeometryVertex::new(value),
            fragment: ShaderTemplateGeometryFragment::new(value),
        })
    }
}

impl ShaderTemplateGeometry {
    pub fn into_source(self) -> Result<String> {
        let vertex_source = self.vertex.render()?;
        let fragment_source = self.fragment.render()?;
        let source = format!("{}\n{}", vertex_source, fragment_source);

        // print_shader_source(&vertex_source, false);
        //print_shader_source(&source, false);

        Ok(source)
    }

    #[cfg(debug_assertions)]
    pub fn debug_label(&self) -> Option<&str> {
        Some("Geometry")
    }
}
