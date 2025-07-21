use crate::shaders::fragment::entry::{
    debug_normals::ShaderCacheKeyFragmentDebugNormals, pbr::ShaderCacheKeyFragmentPbr,
    post_process::ShaderCacheKeyFragmentPostProcess,
};

// Just a cache key to identify the shader
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderCacheKeyFragment {
    Pbr(ShaderCacheKeyFragmentPbr),
    PostProcess(ShaderCacheKeyFragmentPostProcess),
    DebugNormals(ShaderCacheKeyFragmentDebugNormals),
}
