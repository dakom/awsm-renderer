use awsm_renderer_core::pipeline::primitive::IndexFormat;

use crate::{
    buffer::helpers::{u8_to_f32_vec, u8_to_i16_vec, u8_to_i8_vec, u8_to_u16_vec, u8_to_u32_vec}, gltf::buffers::MeshBufferIndexInfoWithOffset, mesh::MeshBufferIndexInfo
};

use super::{accessor::accessor_to_bytes, AwsmGltfError, Result};

#[derive(Debug, Clone)]
pub struct GltfMeshBufferIndexInfo {
    // offset in index_bytes where this primitive starts
    pub offset: usize,
    // number of index elements for this primitive
    pub count: usize,
    // number of bytes per index (e.g. 2 for u16, 4 for u32)
    pub data_size: usize,
    // the format of the index data
    pub format: IndexFormat,
}

impl GltfMeshBufferIndexInfo {
    // the size in bytes of the index buffer for this primitive
    pub fn total_size(&self) -> usize {
        self.count * self.data_size
    }
}

impl From<GltfMeshBufferIndexInfo> for MeshBufferIndexInfoWithOffset {
    fn from(info: GltfMeshBufferIndexInfo) -> Self {
        Self {
            offset: info.offset,
            count: info.count,
            data_size: info.data_size,
            format: info.format,
        }
    }
}

impl GltfMeshBufferIndexInfo {
    pub fn maybe_new(
        primitive: &gltf::Primitive<'_>,
        buffers: &[Vec<u8>],
        index_bytes: &mut Vec<u8>,
    ) -> Result<Option<Self>> {
        match primitive.indices() {
            None => Ok(None),
            Some(accessor) => {
                let offset = index_bytes.len();
                let accessor_bytes = accessor_to_bytes(&accessor, buffers)?;

                let format = match accessor.data_type() {
                    // https://docs.rs/web-sys/latest/web_sys/enum.GpuIndexFormat.html
                    gltf::accessor::DataType::U16 => IndexFormat::Uint16,
                    gltf::accessor::DataType::U32 => IndexFormat::Uint32,
                    // Only Uint16 and Uint16 are supported for indices
                    // these are convered
                    gltf::accessor::DataType::I16 => IndexFormat::Uint16,
                    gltf::accessor::DataType::I8 => IndexFormat::Uint16,
                    gltf::accessor::DataType::U8 => IndexFormat::Uint16,
                    // Floats for indices is probably a mistake
                    gltf::accessor::DataType::F32 => {
                        return Err(AwsmGltfError::UnsupportedIndexDataType(
                            accessor.data_type(),
                        ))
                    }
                };

                let data_size = match format {
                    IndexFormat::Uint16 => 2,
                    IndexFormat::Uint32 => 4,
                    _ => {
                        return Err(AwsmGltfError::UnsupportedIndexDataType(
                            accessor.data_type(),
                        ))
                    }
                };


                match accessor.data_type() {
                    gltf::accessor::DataType::U16 | gltf::accessor::DataType::U32 => {
                        index_bytes.extend_from_slice(&accessor_bytes);
                    }
                    gltf::accessor::DataType::I16 => {
                        let i16_values = u8_to_i16_vec(&accessor_bytes);
                        for value in i16_values {
                            if value < 0 {
                                return Err(AwsmGltfError::ConstructNormals(
                                    format!("Negative index value: {}", value)
                                ));
                            }
                            index_bytes.extend_from_slice(&(value as u16).to_le_bytes());
                        }
                    }
                    gltf::accessor::DataType::I8 => {
                        for byte in accessor_bytes.iter() {
                            let i8_value = *byte as i8;
                            if i8_value < 0 {
                                return Err(AwsmGltfError::ConstructNormals(
                                    format!("Negative index value: {}", i8_value)
                                ));
                            }
                            index_bytes.extend_from_slice(&(i8_value as u16).to_le_bytes());
                        }
                    }
                    gltf::accessor::DataType::U8 => {
                        for byte in accessor_bytes.iter() {
                            index_bytes.extend_from_slice(&(*byte as u16).to_le_bytes());
                        }
                    },

                    gltf::accessor::DataType::F32 => {
                        let f32_values = u8_to_f32_vec(&accessor_bytes);
                        for value in f32_values {
                            let value = value as u32;
                            index_bytes.extend_from_slice(&value.to_le_bytes());
                        }
                    }
                }


                let info = Self {
                    offset,
                    count: accessor.count(),
                    data_size,
                    format,
                };

                assert_eq!(index_bytes.len() - offset, info.total_size());

                Ok(Some(info))
            }
        }
    }
}

pub fn generate_fresh_indices_from_primitive(
    primitive: &gltf::Primitive,
    index_bytes: &mut Vec<u8>,
) -> Result<MeshBufferIndexInfoWithOffset> {
    let offset = index_bytes.len();
    // Get vertex count from any attribute (positions is guaranteed to exist)
    let vertex_count = primitive
        .attributes()
        .next()
        .map(|(_, accessor)| accessor.count())
        .unwrap_or(0);

    if vertex_count == 0 {
        return Ok(MeshBufferIndexInfoWithOffset {
            offset,
            count: 0,
            data_size: 4, // u32
            format: IndexFormat::Uint32,
        });
    }

    // Check if we can use u16 or need u32
    let (format, data_size) = if vertex_count <= u16::MAX as usize {
        (IndexFormat::Uint16, 2)
    } else {
        (IndexFormat::Uint32, 4)
    };

    let start_offset = index_bytes.len();

    // Generate sequential indices based on primitive mode
    match primitive.mode() {
        gltf::mesh::Mode::Triangles => {
            // Simple case: 0, 1, 2, 3, 4, 5, ...
            let index_count = vertex_count;
            for i in 0..index_count {
                match format {
                    IndexFormat::Uint16 => {
                        index_bytes.extend_from_slice(&(i as u16).to_le_bytes());
                    }
                    IndexFormat::Uint32 => {
                        index_bytes.extend_from_slice(&(i as u32).to_le_bytes());
                    },
                    _ => {
                        return Err(AwsmGltfError::UnsupportedIndexFormat(format));
                    }
                }
            }
            
            Ok(MeshBufferIndexInfoWithOffset {
                offset,
                count: index_count,
                data_size,
                format,
            })
        }
        gltf::mesh::Mode::TriangleStrip => {
            // Convert triangle strip to triangle list
            if vertex_count < 3 {
                return Err(AwsmGltfError::UnsupportedIndexMode(
                    "Triangle strip needs at least 3 vertices".to_string(),
                ));
            }

            let triangle_count = vertex_count - 2;
            let index_count = triangle_count * 3;

            for i in 0..triangle_count {
                let (v0, v1, v2) = if i % 2 == 0 {
                    // Even triangles: normal winding
                    (i, i + 1, i + 2)
                } else {
                    // Odd triangles: reverse winding to maintain consistent orientation
                    (i, i + 2, i + 1)
                };

                match format {
                    IndexFormat::Uint16 => {
                        index_bytes.extend_from_slice(&(v0 as u16).to_le_bytes());
                        index_bytes.extend_from_slice(&(v1 as u16).to_le_bytes());
                        index_bytes.extend_from_slice(&(v2 as u16).to_le_bytes());
                    }
                    IndexFormat::Uint32 => {
                        index_bytes.extend_from_slice(&(v0 as u32).to_le_bytes());
                        index_bytes.extend_from_slice(&(v1 as u32).to_le_bytes());
                        index_bytes.extend_from_slice(&(v2 as u32).to_le_bytes());
                    },
                    _ => {
                        return Err(AwsmGltfError::UnsupportedIndexFormat(format));
                    }
                }
            }

            Ok(MeshBufferIndexInfoWithOffset {
                offset,
                count: index_count,
                data_size,
                format,
            })
        }
        gltf::mesh::Mode::TriangleFan => {
            // Convert triangle fan to triangle list
            if vertex_count < 3 {
                return Err(AwsmGltfError::UnsupportedIndexMode(
                    "Triangle fan needs at least 3 vertices".to_string(),
                ));
            }

            let triangle_count = vertex_count - 2;
            let index_count = triangle_count * 3;

            for i in 0..triangle_count {
                // Fan triangles: (0, i+1, i+2)
                let (v0, v1, v2) = (0, i + 1, i + 2);

                match format {
                    IndexFormat::Uint16 => {
                        index_bytes.extend_from_slice(&(v0 as u16).to_le_bytes());
                        index_bytes.extend_from_slice(&(v1 as u16).to_le_bytes());
                        index_bytes.extend_from_slice(&(v2 as u16).to_le_bytes());
                    }
                    IndexFormat::Uint32 => {
                        index_bytes.extend_from_slice(&(v0 as u32).to_le_bytes());
                        index_bytes.extend_from_slice(&(v1 as u32).to_le_bytes());
                        index_bytes.extend_from_slice(&(v2 as u32).to_le_bytes());
                    },
                    _ => {
                        return Err(AwsmGltfError::UnsupportedIndexFormat(format));
                    }
                }
            }

            Ok(MeshBufferIndexInfoWithOffset {
                offset,
                count: index_count,
                data_size,
                format,
            })
        }
        other => Err(AwsmGltfError::UnsupportedIndexMode(format!(
            "Primitive mode {:?} not supported for visibility buffer rendering",
            other
        ))),
    }
}
