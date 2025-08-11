use awsm_renderer_core::pipeline::{
    primitive::{FrontFace, IndexFormat},
    vertex::VertexFormat,
};
use gltf::{
    accessor::{DataType, Dimensions},
    Semantic,
};

use crate::{
    gltf::{
        buffers::MeshBufferIndexInfoWithOffset,
        error::{AwsmGltfError, Result},
    },
    mesh::{MeshBufferIndexInfo, MeshBufferVertexAttributeKind},
};

pub(super) fn extract_triangle_indices(
    index: &MeshBufferIndexInfoWithOffset,
    all_index_bytes: &[u8],
) -> Result<Vec<[usize; 3]>> {
    if index.count % 3 != 0 {
        return Err(AwsmGltfError::ExtractIndices(format!(
            "Index count ({}) is not a multiple of 3, cannot form triangles.",
            index.count
        )));
    }

    if index.count == 0 {
        return Ok(Vec::new());
    }

    // we're just working with the bytes of this primitive
    let index_bytes = &all_index_bytes[index.offset..index.offset + index.total_size()];

    let num_triangles = index.count / 3;
    let mut triangles = Vec::with_capacity(num_triangles);

    for i in 0..num_triangles {
        let mut triangle = [0usize; 3];

        for j in 0..3 {
            let index_offset = (i * 3 + j) * index.data_size;

            if index_offset + index.data_size > index_bytes.len() {
                return Err(AwsmGltfError::ExtractIndices(format!(
                    "Index data out of bounds at triangle {}, vertex {}",
                    i, j
                )));
            }

            let index_slice = &index_bytes[index_offset..index_offset + index.data_size];

            let vertex_idx = match index.format {
                IndexFormat::Uint16 => {
                    if index.data_size != 2 {
                        return Err(AwsmGltfError::ExtractIndices(
                            "IndexFormat::Uint16 expects data_size of 2".to_string(),
                        ));
                    }
                    u16::from_le_bytes(index_slice.try_into().unwrap()) as usize
                }
                IndexFormat::Uint32 => {
                    if index.data_size != 4 {
                        return Err(AwsmGltfError::ExtractIndices(
                            "IndexFormat::Uint32 expects data_size of 4".to_string(),
                        ));
                    }
                    u32::from_le_bytes(index_slice.try_into().unwrap()) as usize
                }
                _ => {
                    return Err(AwsmGltfError::ConstructNormals(format!(
                        "Unsupported index format: {:?}",
                        index.format
                    )));
                }
            };

            triangle[j] = vertex_idx;
        }

        triangles.push(triangle);
    }

    Ok(triangles)
}

pub(super) fn get_position_from_buffer(positions: &[u8], vertex_index: usize) -> Result<[f32; 3]> {
    let offset = vertex_index * 12; // 3 f32s = 12 bytes

    let vertex_count = positions.len() / 12;
    if vertex_index >= vertex_count {
        return Err(AwsmGltfError::Positions(format!(
            "Position data out of bounds for vertex {}. Buffer has {} vertices ({} bytes), requested vertex {}", 
            vertex_index, vertex_count, positions.len(), vertex_index
        )));
    }

    if offset + 12 > positions.len() {
        return Err(AwsmGltfError::Positions(format!(
            "Position data out of bounds for vertex {}. Offset {} + 12 > buffer size {}",
            vertex_index,
            offset,
            positions.len()
        )));
    }

    // From spec:
    // "All buffer data defined in this specification (i.e., geometry attributes, geometry indices, sparse accessor data, animation inputs and outputs, inverse bind matrices)
    // MUST use little endian byte order."
    let x = f32::from_le_bytes([
        positions[offset],
        positions[offset + 1],
        positions[offset + 2],
        positions[offset + 3],
    ]);
    let y = f32::from_le_bytes([
        positions[offset + 4],
        positions[offset + 5],
        positions[offset + 6],
        positions[offset + 7],
    ]);
    let z = f32::from_le_bytes([
        positions[offset + 8],
        positions[offset + 9],
        positions[offset + 10],
        positions[offset + 11],
    ]);

    Ok([x, y, z])
}

pub(super) fn normalize_triangle_winding(
    triangle: [usize; 3],
    front_face: FrontFace,
) -> [usize; 3] {
    match front_face {
        FrontFace::Ccw => triangle,                               // Keep as-is
        FrontFace::Cw => [triangle[0], triangle[2], triangle[1]], // Flip winding
        _ => {
            // unreachable, but handle just in case
            tracing::warn!(
                "Unexpected winding order, returning original triangle: {:?}",
                triangle
            );
            triangle
        }
    }
}

pub fn transform_to_winding_order(world_matrix: &glam::Mat4) -> FrontFace {
    /*
     From spec: "When a mesh primitive uses any triangle-based topology (i.e., triangles, triangle strip, or triangle fan),
     the determinant of the node’s global transform defines the winding order of that primitive.
     If the determinant is a positive value, the winding order triangle faces is counterclockwise;
     in the opposite case, the winding order is clockwise.
    */
    if world_matrix.determinant() > 0.0 {
        FrontFace::Ccw
    } else {
        FrontFace::Cw
    }
}

pub(super) fn semantic_to_shader_key(semantic: &gltf::Semantic) -> MeshBufferVertexAttributeKind {
    match semantic {
        Semantic::Positions => MeshBufferVertexAttributeKind::Positions,
        Semantic::Normals => MeshBufferVertexAttributeKind::Normals,
        Semantic::Tangents => MeshBufferVertexAttributeKind::Tangents,
        Semantic::Colors(n) => MeshBufferVertexAttributeKind::Colors { count: *n },
        Semantic::TexCoords(n) => MeshBufferVertexAttributeKind::TexCoords { count: *n },
        Semantic::Joints(n) => MeshBufferVertexAttributeKind::Joints { count: *n },
        Semantic::Weights(n) => MeshBufferVertexAttributeKind::Weights { count: *n },
    }
}

pub(super) fn semantic_cmp(a: &gltf::Semantic, b: &gltf::Semantic) -> std::cmp::Ordering {
    fn level_1(semantic: &gltf::Semantic) -> usize {
        match semantic {
            gltf::Semantic::Positions => 1,
            gltf::Semantic::Normals => 2,
            gltf::Semantic::Tangents => 3,
            gltf::Semantic::Colors(_) => 4,
            gltf::Semantic::TexCoords(_) => 5,
            gltf::Semantic::Joints(_) => 6,
            gltf::Semantic::Weights(_) => 7,
        }
    }
    fn level_2(semantic: &gltf::Semantic) -> u32 {
        match semantic {
            gltf::Semantic::Positions => 0,
            gltf::Semantic::Normals => 0,
            gltf::Semantic::Tangents => 0,
            gltf::Semantic::Colors(n) => *n,
            gltf::Semantic::TexCoords(n) => *n,
            gltf::Semantic::Joints(n) => *n,
            gltf::Semantic::Weights(n) => *n,
        }
    }

    let a_level_1 = level_1(a);
    let b_level_1 = level_1(b);
    let a_level_2 = level_2(a);
    let b_level_2 = level_2(b);

    match a_level_1.cmp(&b_level_1) {
        std::cmp::Ordering::Equal => a_level_2.cmp(&b_level_2),
        other => other,
    }
}

pub(super) fn get_attribute_components(attr_kind: &MeshBufferVertexAttributeKind) -> u32 {
    match attr_kind {
        MeshBufferVertexAttributeKind::Positions => 3,
        MeshBufferVertexAttributeKind::Normals => 3,
        MeshBufferVertexAttributeKind::Tangents => 4, // vec4 (tangent + handedness)
        MeshBufferVertexAttributeKind::Colors { .. } => 4, // RGBA
        MeshBufferVertexAttributeKind::TexCoords { .. } => 2, // UV
        MeshBufferVertexAttributeKind::Joints { .. } => 4, // 4 joint indices
        MeshBufferVertexAttributeKind::Weights { .. } => 4, // 4 weights
    }
}

pub(super) fn accessor_vertex_format(
    data_type: DataType,
    dimensions: Dimensions,
    normalized: bool,
) -> VertexFormat {
    // https://gpuweb.github.io/gpuweb/#enumdef-gpuvertexformat
    match (data_type, dimensions, normalized) {
        // I8: normalized → signed normalized formats; not normalized → signed integer formats.
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Scalar, true) => {
            VertexFormat::Snorm8
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Scalar, false) => {
            VertexFormat::Sint8
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec2, true) => {
            VertexFormat::Snorm8x2
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec2, false) => {
            VertexFormat::Sint8x2
        }
        // Vec3 is not directly supported; pad to 4.
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec3, true) => {
            VertexFormat::Snorm8x4
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec3, false) => {
            VertexFormat::Sint8x4
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec4, true) => {
            VertexFormat::Snorm8x4
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec4, false) => {
            VertexFormat::Sint8x4
        }
        // For matrices, treat as the corresponding vector type (i.e. a Mat2 becomes a Vec2, etc.)
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat2, true) => {
            VertexFormat::Snorm8x2
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat2, false) => {
            VertexFormat::Sint8x2
        }
        // Mat3 has 3 columns; pad each column to 4 components.
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat3, true) => {
            VertexFormat::Snorm8x4
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat3, false) => {
            VertexFormat::Sint8x4
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat4, true) => {
            VertexFormat::Snorm8x4
        }
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat4, false) => {
            VertexFormat::Sint8x4
        }

        // U8: normalized → unsigned normalized formats; not normalized → unsigned integer formats.
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Scalar, true) => {
            VertexFormat::Unorm8
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Scalar, false) => {
            VertexFormat::Uint8
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec2, true) => {
            VertexFormat::Unorm8x2
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec2, false) => {
            VertexFormat::Uint8x2
        }
        // Vec3: pad to 4.
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec3, true) => {
            VertexFormat::Unorm8x4
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec3, false) => {
            VertexFormat::Uint8x4
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec4, true) => {
            VertexFormat::Unorm8x4
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec4, false) => {
            VertexFormat::Uint8x4
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat2, true) => {
            VertexFormat::Unorm8x2
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat2, false) => {
            VertexFormat::Uint8x2
        }
        // Mat3: pad to 4.
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat3, true) => {
            VertexFormat::Unorm8x4
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat3, false) => {
            VertexFormat::Uint8x4
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat4, true) => {
            VertexFormat::Unorm8x4
        }
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat4, false) => {
            VertexFormat::Uint8x4
        }

        // I16: normalized → signed normalized; not normalized → signed integer.
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Scalar, true) => {
            VertexFormat::Snorm16
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Scalar, false) => {
            VertexFormat::Sint16
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec2, true) => {
            VertexFormat::Snorm16x2
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec2, false) => {
            VertexFormat::Sint16x2
        }
        // Vec3: pad to 4.
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec3, true) => {
            VertexFormat::Snorm16x4
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec3, false) => {
            VertexFormat::Sint16x4
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec4, true) => {
            VertexFormat::Snorm16x4
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec4, false) => {
            VertexFormat::Sint16x4
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat2, true) => {
            VertexFormat::Snorm16x2
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat2, false) => {
            VertexFormat::Sint16x2
        }
        // Mat3: pad to 4.
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat3, true) => {
            VertexFormat::Snorm16x4
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat3, false) => {
            VertexFormat::Sint16x4
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat4, true) => {
            VertexFormat::Snorm16x4
        }
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat4, false) => {
            VertexFormat::Sint16x4
        }

        // U16: normalized → unsigned normalized; not normalized → unsigned integer.
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Scalar, true) => {
            VertexFormat::Unorm16
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Scalar, false) => {
            VertexFormat::Uint16
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec2, true) => {
            VertexFormat::Unorm16x2
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec2, false) => {
            VertexFormat::Uint16x2
        }
        // Vec3: pad to 4.
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec3, true) => {
            VertexFormat::Unorm16x4
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec3, false) => {
            VertexFormat::Uint16x4
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec4, true) => {
            VertexFormat::Unorm16x4
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec4, false) => {
            VertexFormat::Uint16x4
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat2, true) => {
            VertexFormat::Unorm16x2
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat2, false) => {
            VertexFormat::Uint16x2
        }
        // Mat3: pad to 4.
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat3, true) => {
            VertexFormat::Unorm16x4
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat3, false) => {
            VertexFormat::Uint16x4
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat4, true) => {
            VertexFormat::Unorm16x4
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat4, false) => {
            VertexFormat::Uint16x4
        }

        // U32: normalized flag is ignored.
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Scalar, _) => {
            VertexFormat::Uint32
        }
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Vec2, _) => {
            VertexFormat::Uint32x2
        }
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Vec3, _) => {
            VertexFormat::Uint32x3
        }
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Vec4, _) => {
            VertexFormat::Uint32x4
        }
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Mat2, _) => {
            VertexFormat::Uint32x2
        }
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Mat3, _) => {
            VertexFormat::Uint32x3
        }
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Mat4, _) => {
            VertexFormat::Uint32x4
        }

        // F32: normalized flag is ignored.
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Scalar, _) => {
            VertexFormat::Float32
        }
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Vec2, _) => {
            VertexFormat::Float32x2
        }
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Vec3, _) => {
            VertexFormat::Float32x3
        }
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Vec4, _) => {
            VertexFormat::Float32x4
        }
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Mat2, _) => {
            VertexFormat::Float32x2
        }
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Mat3, _) => {
            VertexFormat::Float32x3
        }
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Mat4, _) => {
            VertexFormat::Float32x4
        }
    }
}
