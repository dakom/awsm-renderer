use awsm_renderer_core::pipeline::primitive::IndexFormat;

use crate::mesh::MeshBufferIndexInfo;

use super::{accessor::accessor_to_bytes, AwsmGltfError, Result};

#[derive(Debug, Clone)]
pub struct GltfMeshBufferIndexInfo {
    // offset in index_bytes where this primitive starts
    pub offset: usize,
    // number of index elements for this primitive
    pub count: usize,
    // number of bytes per index (e.g. 2 for u16, 4 for u32)
    pub stride: usize,
    // total size in bytes of this index buffer
    pub size: usize,

    // the format of the index data
    pub format: IndexFormat,
}

impl From<GltfMeshBufferIndexInfo> for MeshBufferIndexInfo {
    fn from(info: GltfMeshBufferIndexInfo) -> Self {
        Self {
            count: info.count,
            stride: info.stride,
            size: info.size,
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
                let accessor_bytes = accessor_to_bytes(&accessor, &buffers)?;
                index_bytes.extend_from_slice(&accessor_bytes);

                Ok(Some(Self {
                    offset,
                    size: index_bytes.len() - offset,
                    count: accessor.count(),
                    stride: accessor.size(),
                    format: match accessor.data_type() {
                        // https://rustwasm.github.io/wasm-bindgen/api/web_sys/enum.GpuIndexFormat.html
                        gltf::accessor::DataType::U16 => IndexFormat::Uint16,
                        gltf::accessor::DataType::U32 => IndexFormat::Uint32,
                        _ => {
                            return Err(AwsmGltfError::UnsupportedIndexDataType(
                                accessor.data_type(),
                            ))
                        }
                    },
                }))
            }
        }
    }
}
