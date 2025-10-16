use crate::mesh::{MeshBufferInfo, MeshBufferVertexAttributeInfo};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShaderMaterialOpaqueVertexAttributes {
    pub normals: bool,
    pub tangents: bool,
    pub color_sets: Option<u32>,
    /// Number of distinct `TEXCOORD_n` sets present on the mesh.
    ///
    /// Stored as (highest index + 1) so `Some(1)` means only `TEXCOORD_0` exists.
    pub uv_sets: Option<u32>,
}

impl From<&MeshBufferInfo> for ShaderMaterialOpaqueVertexAttributes {
    fn from(mesh_buffer_info: &MeshBufferInfo) -> Self {
        let mut _self = Self::default();

        for attr in &mesh_buffer_info.triangles.vertex_attributes {
            match attr {
                MeshBufferVertexAttributeInfo::Positions { .. } => {
                    // not part of material shader requiremenets
                }
                MeshBufferVertexAttributeInfo::Normals { .. } => {
                    _self.normals = true;
                }
                MeshBufferVertexAttributeInfo::Tangents { .. } => {
                    _self.tangents = true;
                }
                MeshBufferVertexAttributeInfo::Colors { count, .. } => {
                    _self.color_sets = Some(*count + 1);
                }
                MeshBufferVertexAttributeInfo::TexCoords { count, .. } => {
                    // `count` is the zero-based TEXCOORD set index from glTF,
                    // so promote it to a human-friendly "number of sets".
                    _self.uv_sets = Some(*count + 1);
                }
                MeshBufferVertexAttributeInfo::Joints { .. } => {
                    // not part of material shader requirements
                }
                MeshBufferVertexAttributeInfo::Weights { .. } => {
                    // not part of material shader requirements
                }
            }
        }

        _self
    }
}
