use crate::shaders::vertex::entry::mesh::ShaderCacheKeyVertexMesh;

// Just a cache key to identify the shader
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum ShaderCacheKeyVertex {
    Mesh(ShaderCacheKeyVertexMesh),
    Quad,
}

impl ShaderCacheKeyVertex {
    pub fn as_mesh(&self) -> &ShaderCacheKeyVertexMesh {
        match self {
            ShaderCacheKeyVertex::Mesh(cache_key) => cache_key,
            ShaderCacheKeyVertex::Quad => panic!("ShaderCacheKeyVertex::Quad does not have a mesh cache key"),
        }
    }
}
