use std::collections::{BTreeMap, HashMap};

use awsm_renderer_core::pipeline::vertex::VertexFormat;

use crate::mesh::MeshBufferVertexInfo;

use super::accessor::accessor_to_bytes;
use super::Result;

#[derive(Default, Debug, Clone)]
pub struct GltfMeshBufferVertexInfo {
    // offset in vertex_bytes where this primitive starts
    pub offset: usize,
    // number of vertices for this primitive
    pub count: usize,
    // total size in bytes of this vertex
    // same as vertex_count * sum_of_all_vertex_attribute_stride_sizes
    pub size: usize,
    // size of each individual vertex attribute stride
    pub attribute_stride_sizes: HashMap<gltf::Semantic, usize>,
}

impl From<GltfMeshBufferVertexInfo> for MeshBufferVertexInfo {
    fn from(info: GltfMeshBufferVertexInfo) -> Self {
        Self {
            count: info.count,
            size: info.size,
        }
    }
}

impl GltfMeshBufferVertexInfo {
    pub fn new(
        primitive: &gltf::Primitive<'_>,
        buffers: &[Vec<u8>],
        vertex_bytes: &mut Vec<u8>,
    ) -> Result<Self> {
        let offset = vertex_bytes.len();

        let mut attributes: Vec<(gltf::Semantic, gltf::Accessor<'_>)> =
            primitive.attributes().collect();

        attributes.sort_by(|(a, _), (b, _)| semantic_cmp(a, b));

        // this should never be empty, but let's be safe
        let vertex_count = attributes
            .first()
            .map(|(_, accessor)| accessor.count())
            .unwrap_or(0);

        // first we need to read the whole accessor. This will be zero-copy unless one of these is true:
        // 1. they're sparse and we need to replace values
        // 2. there's no view, and we need to fill it with zeroes
        //
        // otherwise, it's just a slice of the original buffer
        //
        // We need to collect the attributes that have the same discriminant into one buffer
        // but different discriminants go into different buffers
        let (attributes_bytes, attribute_stride_sizes) = {
            let mut attribute_stride_sizes = HashMap::new();
            let mut attributes_bytes = Vec::new();

            let mut buffers_by_semantic: BTreeMap<gltf::Semantic, (Vec<u8>, usize)> =
                BTreeMap::new();

            for (semantic, accessor) in attributes {
                let entry = buffers_by_semantic
                    .entry(semantic)
                    .or_insert_with(|| (Vec::new(), 0));

                let bytes = accessor_to_bytes(&accessor, buffers)?;
                entry.0.extend_from_slice(&bytes);
                entry.1 += accessor.size();
            }

            for (semantic, (bytes, stride)) in buffers_by_semantic {
                attribute_stride_sizes.insert(semantic, stride);
                attributes_bytes.push((bytes, stride));
            }

            (attributes_bytes, attribute_stride_sizes)
        };

        // now let's predictably interleave the attributes into our final vertex buffer
        // this does extend/copy the data, but it saves us additional calls at render time
        for vertex in 0..vertex_count {
            for (attribute_bytes, attribute_stride_size) in attributes_bytes.iter() {
                let attribute_byte_offset = vertex * attribute_stride_size;
                let attribute_bytes = &attribute_bytes
                    [attribute_byte_offset..attribute_byte_offset + attribute_stride_size];

                vertex_bytes.extend_from_slice(attribute_bytes);
            }
        }

        Ok(Self {
            offset,
            count: vertex_count,
            size: vertex_bytes.len() - offset,
            attribute_stride_sizes,
        })
    }
}

fn semantic_cmp(a: &gltf::Semantic, b: &gltf::Semantic) -> std::cmp::Ordering {
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

pub fn accessor_vertex_format(accessor: &gltf::Accessor<'_>) -> VertexFormat {
    // https://gpuweb.github.io/gpuweb/#enumdef-gpuvertexformat
    match (
        accessor.data_type(),
        accessor.dimensions(),
        accessor.normalized(),
    ) {
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
