use awsm_renderer_core::pipeline::vertex::VertexFormat;
use gltf::accessor::{DataType, Dimensions};
use gltf::Semantic;
use std::collections::HashMap;

use crate::buffer::helpers::{u8_to_i16_vec, u8_to_u16_vec};
use crate::gltf::buffers::normals::compute_normals;
use crate::gltf::error::AwsmGltfError;
use crate::mesh::{MeshBufferIndexInfo, MeshBufferVertexInfo};
use crate::render_passes::geometry::shader::cache_key::ShaderCacheKeyGeometryAttribute;

use super::accessor::accessor_to_bytes;
use super::index::GltfMeshBufferIndexInfo;
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
    pub attributes: Vec<MeshBufferVertexAttribute>,
}

impl From<GltfMeshBufferVertexInfo> for MeshBufferVertexInfo {
    fn from(info: GltfMeshBufferVertexInfo) -> Self {
        Self {
            count: info.count,
            size: info.size,
            attributes: info.attributes,
        }
    }
}
pub(super) struct BufferByAttributeKind {
    pub attribute_bytes: Vec<u8>,
    pub attribute_size: usize,
    pub vertex_format: VertexFormat,
    pub attribute_kind_count: u32,
}

impl GltfMeshBufferVertexInfo {
    pub fn new(
        primitive: &gltf::Primitive<'_>,
        buffers: &[Vec<u8>],
        index: (&MeshBufferIndexInfo, &[u8]),
        vertex_bytes: &mut Vec<u8>,
    ) -> Result<Self> {
        let offset = vertex_bytes.len();

        let mut gltf_attributes: Vec<(gltf::Semantic, gltf::Accessor<'_>)> =
            primitive.attributes().collect();

        gltf_attributes.sort_by(|(a, _), (b, _)| semantic_cmp(a, b));

        // this should never be empty, but let's be safe
        let vertex_count = gltf_attributes
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
        let attributes = {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            enum TempAttributeKind {
                Positions,
                Normals,
                Tangents,
                Colors,
                TexCoords,
                Joints,
                Weights,
            }

            impl From<Semantic> for TempAttributeKind {
                fn from(semantic: Semantic) -> Self {
                    match semantic {
                        Semantic::Positions => TempAttributeKind::Positions,
                        Semantic::Normals => TempAttributeKind::Normals,
                        Semantic::Tangents => TempAttributeKind::Tangents,
                        Semantic::Colors(_) => TempAttributeKind::Colors,
                        Semantic::TexCoords(_) => TempAttributeKind::TexCoords,
                        Semantic::Joints(_) => TempAttributeKind::Joints,
                        Semantic::Weights(_) => TempAttributeKind::Weights,
                    }
                }
            }

            let mut buffers_by_attribute_kind = HashMap::new();

            for (semantic, accessor) in gltf_attributes {
                let entry = buffers_by_attribute_kind
                    .entry(TempAttributeKind::from(semantic))
                    .or_insert_with(|| {
                        BufferByAttributeKind {
                            attribute_bytes: Vec::new(),
                            attribute_size: 0usize,
                            vertex_format: accessor_vertex_format(
                                // wgsl doesn't work with 16-bit, so we need to convert to 32-bit
                                match accessor.data_type() {
                                    DataType::U16 => DataType::U32,
                                    DataType::I16 => DataType::U32,
                                    other => other,
                                },
                                accessor.dimensions(),
                                accessor.normalized(),
                            ),
                            attribute_kind_count: 0u32,
                        }
                    });

                entry.attribute_size = match accessor.data_type() {
                    gltf::accessor::DataType::U16 | gltf::accessor::DataType::I16 => {
                        // 2 bytes per element
                        accessor.size() * 2
                    }
                    _ => accessor.size(),
                };

                entry.attribute_kind_count += 1;

                let bytes = accessor_to_bytes(&accessor, buffers)?;
                // wgsl doesn't work with 16-bit, so we need to convert to 32-bit
                match accessor.data_type() {
                    gltf::accessor::DataType::U16 => {
                        let values: Vec<u32> = u8_to_u16_vec(&bytes)
                            .into_iter()
                            .map(|v| v.into())
                            .collect();

                        let bytes = unsafe {
                            std::slice::from_raw_parts(
                                values.as_ptr() as *const u8,
                                values.len() * std::mem::size_of::<u32>(),
                            )
                        };

                        entry.attribute_bytes.extend_from_slice(bytes);
                    }
                    gltf::accessor::DataType::I16 => {
                        let values: Vec<i32> = u8_to_i16_vec(&bytes)
                            .into_iter()
                            .map(|v| v.into())
                            .collect();
                        let bytes = unsafe {
                            std::slice::from_raw_parts(
                                values.as_ptr() as *const u8,
                                values.len() * std::mem::size_of::<i32>(),
                            )
                        };
                        entry.attribute_bytes.extend_from_slice(bytes);
                    }
                    _ => {
                        entry.attribute_bytes.extend_from_slice(&bytes);
                    }
                }
            }

            // no built-in normals? compute them...
            if !buffers_by_attribute_kind.contains_key(&TempAttributeKind::Normals) {
                let positions = buffers_by_attribute_kind
                    .get(&TempAttributeKind::Positions)
                    .ok_or_else(|| {
                        AwsmGltfError::ConstructNormals("missing positions".to_string())
                    })?;
                let normals_bytes = compute_normals(positions, index)?;

                buffers_by_attribute_kind.insert(
                    TempAttributeKind::Normals,
                    BufferByAttributeKind {
                        attribute_bytes: normals_bytes,
                        attribute_size: positions.attribute_size,
                        vertex_format: positions.vertex_format,
                        attribute_kind_count: positions.attribute_kind_count,
                    },
                );
            }

            let mut attributes: Vec<(Vec<u8>, MeshBufferVertexAttribute)> = Vec::new();
            let mut offset = 0;
            for (kind, data) in buffers_by_attribute_kind.into_iter() {
                let BufferByAttributeKind {
                    attribute_bytes,
                    attribute_size,
                    vertex_format,
                    attribute_kind_count,
                } = data;
                attributes.push((
                    attribute_bytes,
                    MeshBufferVertexAttribute {
                        size: attribute_size,
                        offset,
                        format: vertex_format,
                        shader_key_kind: match kind {
                            TempAttributeKind::Positions => {
                                ShaderCacheKeyGeometryAttribute::Positions
                            }
                            TempAttributeKind::Normals => ShaderCacheKeyGeometryAttribute::Normals,
                            TempAttributeKind::Tangents => {
                                ShaderCacheKeyGeometryAttribute::Tangents
                            }
                            TempAttributeKind::Colors => ShaderCacheKeyGeometryAttribute::Colors {
                                count: attribute_kind_count,
                            },
                            TempAttributeKind::TexCoords => {
                                ShaderCacheKeyGeometryAttribute::TexCoords {
                                    count: attribute_kind_count,
                                }
                            }
                            TempAttributeKind::Joints => ShaderCacheKeyGeometryAttribute::Joints {
                                count: attribute_kind_count,
                            },
                            TempAttributeKind::Weights => {
                                ShaderCacheKeyGeometryAttribute::Weights {
                                    count: attribute_kind_count,
                                }
                            }
                        },
                    },
                ));

                offset += attribute_size;
            }

            attributes.sort_by(|(_, a), (_, b)| {
                fn inner_num(x: &ShaderCacheKeyGeometryAttribute) -> u32 {
                    match x {
                        ShaderCacheKeyGeometryAttribute::Positions => 0,
                        ShaderCacheKeyGeometryAttribute::Normals => 1,
                        ShaderCacheKeyGeometryAttribute::Tangents => 2,
                        ShaderCacheKeyGeometryAttribute::Colors { .. } => 3,
                        ShaderCacheKeyGeometryAttribute::TexCoords { .. } => 4,
                        ShaderCacheKeyGeometryAttribute::Joints { .. } => 5,
                        ShaderCacheKeyGeometryAttribute::Weights { .. } => 6,
                    }
                }

                inner_num(&a.shader_key_kind).cmp(&inner_num(&b.shader_key_kind))
            });

            attributes
        };

        // now let's predictably interleave the attributes into our final vertex buffer
        // this does extend/copy the data, but it saves us additional calls at render time
        for vertex in 0..vertex_count {
            for (
                attribute_bytes,
                MeshBufferVertexAttribute {
                    size,
                    shader_key_kind,
                    ..
                },
            ) in attributes.iter()
            {
                for i in 0..shader_key_kind.count() {
                    let attribute_byte_offset = vertex * (size * (i as usize + 1));

                    let attribute_bytes =
                        &attribute_bytes[attribute_byte_offset..attribute_byte_offset + size];

                    vertex_bytes.extend_from_slice(attribute_bytes);
                }
            }
        }

        let size = vertex_bytes.len() - offset;
        let attributes = attributes
            .into_iter()
            .map(|(_, attribute)| attribute)
            .collect::<Vec<_>>();

        assert_eq!(
            size,
            vertex_count
                * attributes
                    .iter()
                    .map(|x| (x.size * x.shader_key_kind.count() as usize))
                    .sum::<usize>()
        );

        Ok(Self {
            offset,
            count: vertex_count,
            size,
            attributes,
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

fn accessor_vertex_format(
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
