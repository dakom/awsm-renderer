use crate::{render_passes::shader_cache_key::ShaderCacheKeyRenderPass, shaders::ShaderCacheKey};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyGeometry {
    pub attributes: Vec<ShaderCacheKeyGeometryAttribute>,
    pub morphs: ShaderCacheKeyGeometryMorphs,
    pub has_instance_transforms: bool,
}


impl From<ShaderCacheKeyGeometry> for ShaderCacheKey {
    fn from(key: ShaderCacheKeyGeometry) -> Self {
        ShaderCacheKey::RenderPass(ShaderCacheKeyRenderPass::Geometry(key))
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderCacheKeyGeometryAttribute {
    /// XYZ vertex positions.
    Positions,

    /// XYZ vertex normals.
    Normals,

    /// XYZW vertex tangents where the `w` component is a sign value indicating the
    /// handedness of the tangent basis.
    Tangents,

    /// RGB or RGBA vertex color.
    Colors { count: u32 },

    /// UV texture co-ordinates.
    TexCoords { count: u32 },

    /// Joint indices.
    Joints { count: u32 },

    /// Joint weights.
    Weights { count: u32 },
}

impl ShaderCacheKeyGeometryAttribute {
    pub fn count(&self) -> u32 {
        match self {
            ShaderCacheKeyGeometryAttribute::Positions => 1,
            ShaderCacheKeyGeometryAttribute::Normals => 1,
            ShaderCacheKeyGeometryAttribute::Tangents => 1,
            ShaderCacheKeyGeometryAttribute::Colors { count } => *count,
            ShaderCacheKeyGeometryAttribute::TexCoords { count } => *count,
            ShaderCacheKeyGeometryAttribute::Joints { count } => *count,
            ShaderCacheKeyGeometryAttribute::Weights { count } => *count,
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShaderCacheKeyGeometryMorphs {
    pub position: bool,
    pub normal: bool,
    pub tangent: bool,
}

impl ShaderCacheKeyGeometryMorphs {
    pub fn any(&self) -> bool {
        self.position || self.normal || self.tangent
    }
}