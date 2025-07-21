#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct ShaderCacheKeyVertexMesh {
    pub attributes: Vec<ShaderCacheKeyVertexMeshAttribute>,
    pub morphs: ShaderCacheKeyVertexMeshMorphs,
    pub has_instance_transforms: bool,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderCacheKeyVertexMeshAttribute {
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

impl ShaderCacheKeyVertexMeshAttribute {
    pub fn count(&self) -> u32 {
        match self {
            ShaderCacheKeyVertexMeshAttribute::Positions => 1,
            ShaderCacheKeyVertexMeshAttribute::Normals => 1,
            ShaderCacheKeyVertexMeshAttribute::Tangents => 1,
            ShaderCacheKeyVertexMeshAttribute::Colors { count } => *count,
            ShaderCacheKeyVertexMeshAttribute::TexCoords { count } => *count,
            ShaderCacheKeyVertexMeshAttribute::Joints { count } => *count,
            ShaderCacheKeyVertexMeshAttribute::Weights { count } => *count,
        }
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShaderCacheKeyVertexMeshMorphs {
    pub position: bool,
    pub normal: bool,
    pub tangent: bool,
}

impl ShaderCacheKeyVertexMeshMorphs {
    pub fn any(&self) -> bool {
        self.position || self.normal || self.tangent
    }
}
