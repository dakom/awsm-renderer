use askama::Template;

use crate::shaders::vertex::{entry::{mesh::ShaderTemplateVertexMesh, quad::ShaderTemplateVertexQuad}, ShaderCacheKeyVertex};

impl ShaderTemplateVertex {
    pub fn new(cache_key: &ShaderCacheKeyVertex) -> Self {
        match cache_key {
            ShaderCacheKeyVertex::Mesh(cache_key) => {
                ShaderTemplateVertex::Mesh(ShaderTemplateVertexMesh::new(cache_key))
            }
            ShaderCacheKeyVertex::Quad => ShaderTemplateVertex::Quad(ShaderTemplateVertexQuad::new())
        }
    }
}

// The struct that holds the shader template
#[derive(Debug)]
pub enum ShaderTemplateVertex {
    Mesh(ShaderTemplateVertexMesh),
    Quad(ShaderTemplateVertexQuad),
}

impl ShaderTemplateVertex {
    pub fn render(self) -> askama::Result<String> {
        match self {
            ShaderTemplateVertex::Mesh(mesh) => mesh.render(),
            ShaderTemplateVertex::Quad(quad) => quad.render(),
        }
    }
}


#[derive(Debug)]
pub struct ShaderTemplateVertexLocation {
    pub location: u32,
    pub interpolation: Option<&'static str>,
    pub name: String,
    pub data_type: String,
}

#[derive(Debug)]
pub struct ShaderTemplateVertexToFragmentAssignment {
    pub vertex_name: String,
    pub fragment_name: String,
}