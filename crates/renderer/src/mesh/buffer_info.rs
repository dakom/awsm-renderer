use super::error::{AwsmMeshError, Result};
use awsm_renderer_core::pipeline::{primitive::IndexFormat, vertex::VertexFormat};
use slotmap::new_key_type;

pub struct MeshBufferInfos {
    infos: slotmap::SlotMap<MeshBufferInfoKey, MeshBufferInfo>,
}

impl MeshBufferInfos {
    pub fn new() -> Self {
        Self {
            infos: slotmap::SlotMap::with_key(),
        }
    }

    pub fn insert(&mut self, info: MeshBufferInfo) -> MeshBufferInfoKey {
        self.infos.insert(info)
    }

    pub fn get(&self, key: MeshBufferInfoKey) -> Result<&MeshBufferInfo> {
        self.infos
            .get(key)
            .ok_or(AwsmMeshError::BufferInfoNotFound(key))
    }

    pub fn remove(&mut self, key: MeshBufferInfoKey) -> Option<MeshBufferInfo> {
        self.infos.remove(key)
    }
}

#[derive(Debug, Clone)]
pub struct MeshBufferInfo {
    pub vertex: MeshBufferVertexInfo,
    pub triangles: MeshBufferTriangleInfo,
    pub geometry_morph: Option<MeshBufferGeometryMorphInfo>,
    pub material_morph: Option<MeshBufferMaterialMorphInfo>,
    pub skin: Option<MeshBufferSkinInfo>,
}

#[derive(Debug, Clone)]
pub struct MeshBufferVertexInfo {
    // Number of vertices (triangle_count * 3)
    pub count: usize,
}

impl MeshBufferVertexInfo {
    // We have:
    // - positions (vec3<f32>), 12 bytes per vertex
    // - triangle_index (u32), 4 bytes per vertex
    // - barycentric coordinates (vec2<f32>), 8 bytes per vertex
    // Total size per vertex = 12 + 4 + 8 = 24 bytes
    pub const BYTE_SIZE: usize = 24;
    // 16 floats for transform
    pub const BYTE_SIZE_INSTANCE: usize = 64;

    pub fn size(&self) -> usize {
        self.count * Self::BYTE_SIZE
    }
}

#[derive(Debug, Clone)]
pub struct MeshBufferTriangleInfo {
    // Number of triangles in this primitive
    pub count: usize,
    // Per-vertex indices (3 per triangle, indexing into vertex buffer)
    pub vertex_attribute_indices: MeshBufferAttributeIndexInfo,
    // Per-vertex attribute data (original vertex layout for indexed access)
    pub vertex_attributes: Vec<MeshBufferVertexAttributeInfo>,
    // Total size of all vertex attribute data
    pub vertex_attributes_size: usize,
    // Triangle data buffer (vertex indices + material info per triangle)
    pub triangle_data: MeshBufferTriangleDataInfo,
}

impl MeshBufferTriangleInfo {
    pub fn vertex_attribute_stride(&self) -> usize {
        self.vertex_attributes
            .iter()
            .map(|attr| attr.vertex_size())
            .sum()
    }

    pub fn debug_get_attribute_vec_f32(
        &self,
        info: &MeshBufferVertexAttributeInfo,
        data: &[u8],
    ) -> Vec<Vec<f32>> {
        let mut out = Vec::new();
        let mut offset = 0;
        while offset < data.len() {
            for attr in &self.vertex_attributes {
                if std::mem::discriminant(attr) == std::mem::discriminant(info) {
                    let attr_data = &data[offset..offset + attr.vertex_size()];
                    let mut values = Vec::new();
                    for value in
                        attr_data
                            .chunks(attr.data_size())
                            .map(|chunk| match attr.data_size() {
                                1 => chunk[0] as f32,
                                2 => u16::from_le_bytes(chunk.try_into().unwrap()) as f32,
                                4 => f32::from_le_bytes(chunk.try_into().unwrap()),
                                _ => {
                                    panic!("Unsupported vertex attribute data size for debugging")
                                }
                            })
                    {
                        values.push(value);
                    }

                    out.push(values);
                }

                offset += attr.vertex_size();
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct MeshBufferAttributeIndexInfo {
    // Number of index elements for this primitive (triangle_count * 3)
    pub count: usize,
}

impl MeshBufferAttributeIndexInfo {
    pub fn debug_to_vec(&self, data: &[u8]) -> Vec<Vec<usize>> {
        data.chunks(12)
            .map(|chunk| {
                chunk
                    .chunks(4)
                    .map(|c| u32::from_le_bytes(c.try_into().unwrap()) as usize)
                    .collect()
            })
            .collect()
    }
}

impl MeshBufferAttributeIndexInfo {
    // The size in bytes of the index buffer for this primitive
    pub fn total_size(&self) -> usize {
        self.count * 4 // always u32
    }
}

#[derive(Debug, Clone)]
pub struct MeshBufferTriangleDataInfo {
    // Size per triangle (vertex indices, typically 12 bytes (3 u32 indices))
    pub size_per_triangle: usize,
    // Total size of the triangle data for this mesh
    pub total_size: usize,
}

/// Information about geometry morphs (positions only, exploded for visibility buffer)
#[derive(Debug, Clone)]
pub struct MeshBufferGeometryMorphInfo {
    pub targets_len: usize,
    pub triangle_stride_size: usize, // Size per triangle across all targets (positions only)
    pub values_size: usize,
}

/// Information about material morphs (normals + tangents, non-exploded per-vertex)
#[derive(Debug, Clone)]
pub struct MeshBufferMaterialMorphInfo {
    pub attributes: MeshBufferMaterialMorphAttributes, // Which attributes are present
    pub targets_len: usize,
    pub vertex_stride_size: usize, // Size per original vertex across all targets
    pub values_size: usize,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeshBufferMaterialMorphAttributes {
    pub normal: bool,
    pub tangent: bool,
}

/// Information about skin (indices and weights, exploded per-vertex)
#[derive(Debug, Clone)]
pub struct MeshBufferSkinInfo {
    // 4 joint influences per set
    pub set_count: usize, // Number of skin sets (JOINTS_0/WEIGHTS_0, JOINTS_1/WEIGHTS_1, etc.)

    // Buffer size info
    pub index_weights_size: usize, // Total bytes: exploded_vertices * set_count * 16 (vec4<u32>) * 2 (index and weights)
}

impl MeshBufferInfo {
    // Helper to get triangle count
    pub fn triangle_count(&self) -> usize {
        self.triangles.count
    }

    // Helper to check if we have a specific vertex attribute
    pub fn has_vertex_attribute(&self, attr: MeshBufferVertexAttributeInfo) -> bool {
        self.triangles
            .vertex_attributes
            .iter()
            .any(|a| matches!(a, attr))
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshBufferVertexAttributeInfo {
    /// XYZ vertex positions.
    Positions {
        data_size: usize,
        component_len: usize,
    },

    /// XYZ vertex normals.
    Normals {
        data_size: usize,
        component_len: usize,
    },

    /// XYZW vertex tangents where the `w` component is a sign value indicating the
    /// handedness of the tangent basis.
    Tangents {
        data_size: usize,
        component_len: usize,
    },

    /// RGB or RGBA vertex color.
    Colors {
        count: u32,
        data_size: usize,
        component_len: usize,
    },

    /// UV texture co-ordinates.
    TexCoords {
        count: u32,
        data_size: usize,
        component_len: usize,
    },

    /// Joint indices.
    Joints {
        count: u32,
        data_size: usize,
        component_len: usize,
    },

    /// Joint weights.
    Weights {
        count: u32,
        data_size: usize,
        component_len: usize,
    },
}

impl MeshBufferVertexAttributeInfo {
    fn primary_val(&self) -> usize {
        match self {
            MeshBufferVertexAttributeInfo::Positions { .. } => 0,
            MeshBufferVertexAttributeInfo::Normals { .. } => 1,
            MeshBufferVertexAttributeInfo::Tangents { .. } => 2,
            MeshBufferVertexAttributeInfo::Colors { .. } => 3,
            MeshBufferVertexAttributeInfo::TexCoords { .. } => 4,
            MeshBufferVertexAttributeInfo::Joints { .. } => 5,
            MeshBufferVertexAttributeInfo::Weights { .. } => 6,
        }
    }

    fn secondary_val(&self) -> u32 {
        match self {
            MeshBufferVertexAttributeInfo::Positions { .. } => 0,
            MeshBufferVertexAttributeInfo::Normals { .. } => 0,
            MeshBufferVertexAttributeInfo::Tangents { .. } => 0,
            MeshBufferVertexAttributeInfo::Colors { count, .. } => *count,
            MeshBufferVertexAttributeInfo::TexCoords { count, .. } => *count,
            MeshBufferVertexAttributeInfo::Joints { count, .. } => *count,
            MeshBufferVertexAttributeInfo::Weights { count, .. } => *count,
        }
    }

    pub fn force_data_size(&mut self, new_size: usize) {
        match self {
            MeshBufferVertexAttributeInfo::Positions { data_size, .. } => *data_size = new_size,
            MeshBufferVertexAttributeInfo::Normals { data_size, .. } => *data_size = new_size,
            MeshBufferVertexAttributeInfo::Tangents { data_size, .. } => *data_size = new_size,
            MeshBufferVertexAttributeInfo::Colors { data_size, .. } => *data_size = new_size,
            MeshBufferVertexAttributeInfo::TexCoords { data_size, .. } => *data_size = new_size,
            MeshBufferVertexAttributeInfo::Joints { data_size, .. } => *data_size = new_size,
            MeshBufferVertexAttributeInfo::Weights { data_size, .. } => *data_size = new_size,
        }
    }

    pub fn data_size(&self) -> usize {
        match self {
            MeshBufferVertexAttributeInfo::Positions { data_size, .. } => *data_size,
            MeshBufferVertexAttributeInfo::Normals { data_size, .. } => *data_size,
            MeshBufferVertexAttributeInfo::Tangents { data_size, .. } => *data_size,
            MeshBufferVertexAttributeInfo::Colors { data_size, .. } => *data_size,
            MeshBufferVertexAttributeInfo::TexCoords { data_size, .. } => *data_size,
            MeshBufferVertexAttributeInfo::Joints { data_size, .. } => *data_size,
            MeshBufferVertexAttributeInfo::Weights { data_size, .. } => *data_size,
        }
    }

    pub fn component_len(&self) -> usize {
        match self {
            MeshBufferVertexAttributeInfo::Positions { component_len, .. } => *component_len,
            MeshBufferVertexAttributeInfo::Normals { component_len, .. } => *component_len,
            MeshBufferVertexAttributeInfo::Tangents { component_len, .. } => *component_len,
            MeshBufferVertexAttributeInfo::Colors { component_len, .. } => *component_len,
            MeshBufferVertexAttributeInfo::TexCoords { component_len, .. } => *component_len,
            MeshBufferVertexAttributeInfo::Joints { component_len, .. } => *component_len,
            MeshBufferVertexAttributeInfo::Weights { component_len, .. } => *component_len,
        }
    }

    pub fn vertex_size(&self) -> usize {
        // the count is zero-based (e.g. TEXCOORD_0 = count 0) so we need to add 1
        match self {
            MeshBufferVertexAttributeInfo::Positions {
                component_len,
                data_size,
            } => *component_len * *data_size,
            MeshBufferVertexAttributeInfo::Normals {
                component_len,
                data_size,
            } => *component_len * *data_size,
            MeshBufferVertexAttributeInfo::Tangents {
                component_len,
                data_size,
            } => *component_len * *data_size,
            MeshBufferVertexAttributeInfo::Colors {
                component_len,
                data_size,
                count,
            } => *component_len * *data_size * (*count as usize + 1),
            MeshBufferVertexAttributeInfo::TexCoords {
                component_len,
                data_size,
                count,
            } => *component_len * *data_size * (*count as usize + 1),
            MeshBufferVertexAttributeInfo::Joints {
                component_len,
                data_size,
                count,
            } => *component_len * *data_size * (*count as usize + 1),
            MeshBufferVertexAttributeInfo::Weights {
                component_len,
                data_size,
                count,
            } => *component_len * *data_size * (*count as usize + 1),
        }
    }
}

impl PartialOrd for MeshBufferVertexAttributeInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for MeshBufferVertexAttributeInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.primary_val().cmp(&other.primary_val()) {
            std::cmp::Ordering::Equal => self.secondary_val().cmp(&other.secondary_val()),
            ordering => ordering,
        }
    }
}

new_key_type! {
    pub struct MeshBufferInfoKey;
}
