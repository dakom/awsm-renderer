use awsm_renderer_core::pipeline::primitive::IndexFormat;

use crate::{
    buffer::helpers::{u8_to_i16_vec, u8_to_i8_vec},
    mesh::MeshBufferIndexInfo,
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

impl From<GltfMeshBufferIndexInfo> for MeshBufferIndexInfo {
    fn from(info: GltfMeshBufferIndexInfo) -> Self {
        Self {
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
                        let values: Vec<u16> = u8_to_i16_vec(&accessor_bytes)
                            .into_iter()
                            .map(|v| u16::try_from(v).map_err(|e| e.into()))
                            .collect::<Result<Vec<_>>>()?;
                        let bytes = unsafe {
                            std::slice::from_raw_parts(
                                values.as_ptr() as *const u8,
                                values.len() * data_size,
                            )
                        };
                        index_bytes.extend_from_slice(bytes);
                    }
                    gltf::accessor::DataType::I8 => {
                        let values: Vec<u16> = u8_to_i8_vec(&accessor_bytes)
                            .into_iter()
                            .map(|v| u16::try_from(v).map_err(|e| e.into()))
                            .collect::<Result<Vec<_>>>()?;
                        let bytes = unsafe {
                            std::slice::from_raw_parts(
                                values.as_ptr() as *const u8,
                                values.len() * data_size,
                            )
                        };
                        index_bytes.extend_from_slice(bytes);
                    }
                    gltf::accessor::DataType::U8 => {
                        let values: Vec<u16> = accessor_bytes.iter().map(|v| (*v).into()).collect();
                        let bytes = unsafe {
                            std::slice::from_raw_parts(
                                values.as_ptr() as *const u8,
                                values.len() * data_size,
                            )
                        };
                        index_bytes.extend_from_slice(bytes);
                    }
                    gltf::accessor::DataType::F32 => {
                        return Err(AwsmGltfError::UnsupportedIndexDataType(
                            accessor.data_type(),
                        ))
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
