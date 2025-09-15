use crate::mesh::{MeshBufferInfo, MeshBufferVertexAttributeInfo};

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShaderMaterialOpaqueVertexAttributes {
    pub normals: bool,
    pub tangents: bool,
    pub colors: Option<u32>,
    pub tex_coords: Option<u32>,
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
                    _self.colors = Some(*count + 1);
                }
                MeshBufferVertexAttributeInfo::TexCoords { count, .. } => {
                    _self.tex_coords = Some(*count + 1);
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
