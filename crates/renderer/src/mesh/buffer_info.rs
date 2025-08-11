use awsm_renderer_core::pipeline::{primitive::IndexFormat, vertex::VertexFormat};

#[derive(Debug, Clone)]
pub struct MeshBufferInfo {
    pub vertex: MeshBufferVertexInfo,
    pub triangles: MeshBufferTriangleInfo,
    pub morph: Option<MeshBufferMorphInfo>,
}

#[derive(Debug, Clone)]
pub struct MeshBufferVertexInfo {
    // Number of vertices (triangle_count * 3)
    pub count: usize,
    // We have:
    // - positions (vec3<f32>), 12 bytes per vertex
    // - triangle_id (u32), 4 bytes per vertex
    // - barycentric coordinates (vec2<f32>), 8 bytes per vertex
    // Total size per vertex = 12 + 4 + 8 = 24 bytes
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct MeshBufferTriangleInfo {
    // Number of triangles in this primitive
    pub count: usize,
    // Triangle indices (3 per triangle, indexing into vertex buffer)
    pub indices: MeshBufferIndexInfo,
    // Per-vertex attribute data (original vertex layout for indexed access)
    pub vertex_attributes: Vec<MeshBufferVertexAttributeInfo>,
    // Total size of all vertex attribute data
    pub vertex_attributes_size: usize,
    // Triangle data buffer (vertex indices + material info per triangle)
    pub triangle_data: MeshBufferTriangleDataInfo,
}

#[derive(Debug, Clone)]
pub struct MeshBufferIndexInfo {
    // Number of index elements for this primitive (triangle_count * 3)
    pub count: usize,
    // Number of bytes per index (e.g. 2 for u16, 4 for u32)
    pub data_size: usize,
    // The format of the index data
    pub format: IndexFormat,
}

impl MeshBufferIndexInfo {
    // The size in bytes of the index buffer for this primitive
    pub fn total_size(&self) -> usize {
        self.count * self.data_size
    }
}

#[derive(Debug, Clone)]
pub struct MeshBufferVertexAttributeInfo {
    // Which attribute this represents
    pub kind: MeshBufferVertexAttributeKind,
    // Size per vertex (e.g. 8 for vec2<f32> uvs, 12 bytes for vec3<f32> normals, 16 bytes for vec4<f32> colors)
    pub size_per_vertex: usize,
    // Number of components per vertex attribute (e.g. 2 for vec2<f32> uvs, 3 for vec3<f32> normals, 4 for vec4<f32> colors)
    pub components: u32,
}

#[derive(Debug, Clone)]
pub struct MeshBufferTriangleDataInfo {
    // Size per triangle (vertex indices + material data) - typically 16 bytes (3 u32 indices + 1 u32 material_id)
    pub size_per_triangle: usize,
    // Total size of the triangle data for this mesh
    pub total_size: usize,
}

#[derive(Default, Debug, Clone)]
pub struct MeshBufferMorphInfo {
    // Contains info about the specific attribute targets
    pub attributes: MeshBufferMorphAttributes,
    // The number of morph targets for this primitive
    pub targets_len: usize,
    // Size in bytes of morph data per triangle (across all targets)
    // e.g., if we have 3 targets with positions+normals per triangle vertex:
    // 3 targets * 3 vertices * (3 pos + 3 normal) * 4 bytes = 216 bytes per triangle
    pub triangle_stride_size: usize,
    // Total size in bytes of the entire morph buffer for this mesh
    // triangle_count * triangle_stride_size
    pub values_size: usize,
}

impl MeshBufferInfo {
    // Helper to get triangle count
    pub fn triangle_count(&self) -> usize {
        self.triangles.count
    }

    // Helper to check if we have a specific vertex attribute
    pub fn has_vertex_attribute(&self, attr: MeshBufferVertexAttributeKind) -> bool {
        self.triangles
            .vertex_attributes
            .iter()
            .any(|a| a.kind == attr)
    }
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshBufferVertexAttributeKind {
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

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeshBufferMorphAttributes {
    pub position: bool,
    pub normal: bool,
    pub tangent: bool,
}

impl MeshBufferMorphAttributes {
    pub fn any(&self) -> bool {
        self.position || self.normal || self.tangent
    }
}
