#[derive(Default, Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderCacheKeyFragmentDebugNormals {
    pub has_normals: bool, // actually comes from vertex shader, but affects fragment shader
}

impl ShaderCacheKeyFragmentDebugNormals {
    pub fn new(has_normals: bool) -> Self {
        Self { has_normals }
    }
}