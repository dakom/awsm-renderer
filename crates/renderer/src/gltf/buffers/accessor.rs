use std::borrow::Cow;

use super::Result;
use crate::buffers::helpers::{
    u8_to_f32_vec, u8_to_i16_vec, u8_to_i8_vec, u8_to_u16_vec, u8_to_u32_vec,
};

pub fn accessor_to_bytes<'a>(
    accessor: &gltf::Accessor<'_>,
    buffers: &'a [Vec<u8>],
) -> Result<Cow<'a, [u8]>> {
    let length = accessor.size() * accessor.count();

    let mut buffer: Cow<[u8]> = match accessor.view() {
        Some(view) => {
            let buffer = &buffers[view.buffer().index()];
            let start = accessor.offset() + view.offset();
            let end = start + length;
            Cow::Borrowed(&buffer[start..end])
        }
        None => {
            // gltf spec says if we have no view, fill it with zeroes
            // and these may or may not be overwritten with sparse bytes (and/or extensions)
            Cow::Owned(vec![0; length])
        }
    };

    if let Some(sparse) = accessor.sparse() {
        // will only clone if borrowed
        let buffer = buffer.to_mut();

        let indices = sparse_to_indices(&sparse, buffers);

        let values_buffer_slice = &buffers[sparse.values().view().buffer().index()];
        let values_buffer_slice_start = sparse.values().offset() + sparse.values().view().offset();
        let values_buffer_slice = &values_buffer_slice[values_buffer_slice_start..];

        for (value_index, target_index) in indices.into_iter().enumerate() {
            let value_slice_start = value_index * accessor.size();
            let value_slice =
                &values_buffer_slice[value_slice_start..value_slice_start + accessor.size()];

            let buffer_slice_start = target_index * accessor.size();
            let buffer_slice =
                &mut buffer[buffer_slice_start..buffer_slice_start + accessor.size()];

            buffer_slice.copy_from_slice(value_slice);
        }
    }

    Ok(buffer)
}

fn sparse_to_indices(
    sparse: &gltf::accessor::sparse::Sparse<'_>,
    buffers: &[Vec<u8>],
) -> Vec<usize> {
    let indices_buffer_slice = &buffers[sparse.indices().view().buffer().index()];
    let indices_buffer_slice_start = sparse.indices().offset() + sparse.indices().view().offset();
    let indices_buffer_slice = &indices_buffer_slice[indices_buffer_slice_start..];

    let mut index_offset = 0;
    let index_offset_amount = sparse.indices().index_type().size();

    let mut indices = Vec::with_capacity(sparse.count());

    for _ in 0..sparse.count() {
        // "All buffer data defined in this specification [...] MUST use little endian byte order."
        // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#buffers-and-buffer-views-overview
        let index = match sparse.indices().index_type() {
            gltf::accessor::sparse::IndexType::U8 => {
                let index = indices_buffer_slice[index_offset];
                index as usize
            }
            gltf::accessor::sparse::IndexType::U16 => {
                let index = indices_buffer_slice[index_offset..index_offset + 2]
                    .try_into()
                    .unwrap();
                u16::from_le_bytes(index) as usize
            }
            gltf::accessor::sparse::IndexType::U32 => {
                let index = indices_buffer_slice[index_offset..index_offset + 4]
                    .try_into()
                    .unwrap();
                u32::from_le_bytes(index) as usize
            }
        };
        indices.push(index);
        index_offset += index_offset_amount;
    }

    indices
}

// currently just a helper, not used anywhere
#[allow(dead_code)]
pub fn accessor_to_vec(accessor: &gltf::Accessor<'_>, buffers: &[Vec<u8>]) -> Result<AccessorVec> {
    let bytes = accessor_to_bytes(accessor, buffers)?;

    Ok(match accessor.data_type() {
        gltf::accessor::DataType::I8 => {
            let values = u8_to_i8_vec(&bytes);
            match accessor.dimensions() {
                gltf::accessor::Dimensions::Scalar => AccessorVec::ScalarI8(values),
                gltf::accessor::Dimensions::Vec2 => {
                    AccessorVec::Vec2I8(values.chunks_exact(2).map(|v| [v[0], v[1]]).collect())
                }
                gltf::accessor::Dimensions::Vec3 => AccessorVec::Vec3I8(
                    values.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect(),
                ),
                gltf::accessor::Dimensions::Vec4 => AccessorVec::Vec4I8(
                    values
                        .chunks_exact(4)
                        .map(|v| [v[0], v[1], v[2], v[3]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat2 => AccessorVec::Mat2I8(
                    values
                        .chunks_exact(4)
                        .map(|v| [[v[0], v[1]], [v[2], v[3]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat3 => AccessorVec::Mat3I8(
                    values
                        .chunks_exact(9)
                        .map(|v| [[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat4 => AccessorVec::Mat4I8(
                    values
                        .chunks_exact(16)
                        .map(|v| {
                            [
                                [v[0], v[1], v[2], v[3]],
                                [v[4], v[5], v[6], v[7]],
                                [v[8], v[9], v[10], v[11]],
                                [v[12], v[13], v[14], v[15]],
                            ]
                        })
                        .collect(),
                ),
            }
        }
        gltf::accessor::DataType::U8 => {
            let values = bytes;
            match accessor.dimensions() {
                gltf::accessor::Dimensions::Scalar => AccessorVec::ScalarU8(values.to_vec()),
                gltf::accessor::Dimensions::Vec2 => {
                    AccessorVec::Vec2U8(values.chunks_exact(2).map(|v| [v[0], v[1]]).collect())
                }
                gltf::accessor::Dimensions::Vec3 => AccessorVec::Vec3U8(
                    values.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect(),
                ),
                gltf::accessor::Dimensions::Vec4 => AccessorVec::Vec4U8(
                    values
                        .chunks_exact(4)
                        .map(|v| [v[0], v[1], v[2], v[3]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat2 => AccessorVec::Mat2U8(
                    values
                        .chunks_exact(4)
                        .map(|v| [[v[0], v[1]], [v[2], v[3]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat3 => AccessorVec::Mat3U8(
                    values
                        .chunks_exact(9)
                        .map(|v| [[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat4 => AccessorVec::Mat4U8(
                    values
                        .chunks_exact(16)
                        .map(|v| {
                            [
                                [v[0], v[1], v[2], v[3]],
                                [v[4], v[5], v[6], v[7]],
                                [v[8], v[9], v[10], v[11]],
                                [v[12], v[13], v[14], v[15]],
                            ]
                        })
                        .collect(),
                ),
            }
        }
        gltf::accessor::DataType::I16 => {
            let values = u8_to_i16_vec(&bytes);
            match accessor.dimensions() {
                gltf::accessor::Dimensions::Scalar => AccessorVec::ScalarI16(values.to_vec()),
                gltf::accessor::Dimensions::Vec2 => {
                    AccessorVec::Vec2I16(values.chunks_exact(2).map(|v| [v[0], v[1]]).collect())
                }
                gltf::accessor::Dimensions::Vec3 => AccessorVec::Vec3I16(
                    values.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect(),
                ),
                gltf::accessor::Dimensions::Vec4 => AccessorVec::Vec4I16(
                    values
                        .chunks_exact(4)
                        .map(|v| [v[0], v[1], v[2], v[3]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat2 => AccessorVec::Mat2I16(
                    values
                        .chunks_exact(4)
                        .map(|v| [[v[0], v[1]], [v[2], v[3]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat3 => AccessorVec::Mat3I16(
                    values
                        .chunks_exact(9)
                        .map(|v| [[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat4 => AccessorVec::Mat4I16(
                    values
                        .chunks_exact(16)
                        .map(|v| {
                            [
                                [v[0], v[1], v[2], v[3]],
                                [v[4], v[5], v[6], v[7]],
                                [v[8], v[9], v[10], v[11]],
                                [v[12], v[13], v[14], v[15]],
                            ]
                        })
                        .collect(),
                ),
            }
        }
        gltf::accessor::DataType::U16 => {
            let values = u8_to_u16_vec(&bytes);
            match accessor.dimensions() {
                gltf::accessor::Dimensions::Scalar => AccessorVec::ScalarU16(values.to_vec()),
                gltf::accessor::Dimensions::Vec2 => {
                    AccessorVec::Vec2U16(values.chunks_exact(2).map(|v| [v[0], v[1]]).collect())
                }
                gltf::accessor::Dimensions::Vec3 => AccessorVec::Vec3U16(
                    values.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect(),
                ),
                gltf::accessor::Dimensions::Vec4 => AccessorVec::Vec4U16(
                    values
                        .chunks_exact(4)
                        .map(|v| [v[0], v[1], v[2], v[3]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat2 => AccessorVec::Mat2U16(
                    values
                        .chunks_exact(4)
                        .map(|v| [[v[0], v[1]], [v[2], v[3]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat3 => AccessorVec::Mat3U16(
                    values
                        .chunks_exact(9)
                        .map(|v| [[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat4 => AccessorVec::Mat4U16(
                    values
                        .chunks_exact(16)
                        .map(|v| {
                            [
                                [v[0], v[1], v[2], v[3]],
                                [v[4], v[5], v[6], v[7]],
                                [v[8], v[9], v[10], v[11]],
                                [v[12], v[13], v[14], v[15]],
                            ]
                        })
                        .collect(),
                ),
            }
        }
        gltf::accessor::DataType::U32 => {
            let values = u8_to_u32_vec(&bytes);
            match accessor.dimensions() {
                gltf::accessor::Dimensions::Scalar => AccessorVec::ScalarU32(values.to_vec()),
                gltf::accessor::Dimensions::Vec2 => {
                    AccessorVec::Vec2U32(values.chunks_exact(2).map(|v| [v[0], v[1]]).collect())
                }
                gltf::accessor::Dimensions::Vec3 => AccessorVec::Vec3U32(
                    values.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect(),
                ),
                gltf::accessor::Dimensions::Vec4 => AccessorVec::Vec4U32(
                    values
                        .chunks_exact(4)
                        .map(|v| [v[0], v[1], v[2], v[3]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat2 => AccessorVec::Mat2U32(
                    values
                        .chunks_exact(4)
                        .map(|v| [[v[0], v[1]], [v[2], v[3]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat3 => AccessorVec::Mat3U32(
                    values
                        .chunks_exact(9)
                        .map(|v| [[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat4 => AccessorVec::Mat4U32(
                    values
                        .chunks_exact(16)
                        .map(|v| {
                            [
                                [v[0], v[1], v[2], v[3]],
                                [v[4], v[5], v[6], v[7]],
                                [v[8], v[9], v[10], v[11]],
                                [v[12], v[13], v[14], v[15]],
                            ]
                        })
                        .collect(),
                ),
            }
        }
        gltf::accessor::DataType::F32 => {
            let values = u8_to_f32_vec(&bytes);
            match accessor.dimensions() {
                gltf::accessor::Dimensions::Scalar => AccessorVec::ScalarF32(values.to_vec()),
                gltf::accessor::Dimensions::Vec2 => {
                    AccessorVec::Vec2F32(values.chunks_exact(2).map(|v| [v[0], v[1]]).collect())
                }
                gltf::accessor::Dimensions::Vec3 => AccessorVec::Vec3F32(
                    values.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect(),
                ),
                gltf::accessor::Dimensions::Vec4 => AccessorVec::Vec4F32(
                    values
                        .chunks_exact(4)
                        .map(|v| [v[0], v[1], v[2], v[3]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat2 => AccessorVec::Mat2F32(
                    values
                        .chunks_exact(4)
                        .map(|v| [[v[0], v[1]], [v[2], v[3]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat3 => AccessorVec::Mat3F32(
                    values
                        .chunks_exact(9)
                        .map(|v| [[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]])
                        .collect(),
                ),
                gltf::accessor::Dimensions::Mat4 => AccessorVec::Mat4F32(
                    values
                        .chunks_exact(16)
                        .map(|v| {
                            [
                                [v[0], v[1], v[2], v[3]],
                                [v[4], v[5], v[6], v[7]],
                                [v[8], v[9], v[10], v[11]],
                                [v[12], v[13], v[14], v[15]],
                            ]
                        })
                        .collect(),
                ),
            }
        }
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccessorVec {
    ScalarU8(Vec<u8>),
    ScalarI8(Vec<i8>),
    ScalarU16(Vec<u16>),
    ScalarI16(Vec<i16>),
    ScalarU32(Vec<u32>),
    ScalarF32(Vec<f32>),
    Vec2U8(Vec<[u8; 2]>),
    Vec2I8(Vec<[i8; 2]>),
    Vec2U16(Vec<[u16; 2]>),
    Vec2I16(Vec<[i16; 2]>),
    Vec2U32(Vec<[u32; 2]>),
    Vec2F32(Vec<[f32; 2]>),
    Vec3U8(Vec<[u8; 3]>),
    Vec3I8(Vec<[i8; 3]>),
    Vec3U16(Vec<[u16; 3]>),
    Vec3I16(Vec<[i16; 3]>),
    Vec3U32(Vec<[u32; 3]>),
    Vec3F32(Vec<[f32; 3]>),
    Vec4U8(Vec<[u8; 4]>),
    Vec4I8(Vec<[i8; 4]>),
    Vec4U16(Vec<[u16; 4]>),
    Vec4I16(Vec<[i16; 4]>),
    Vec4U32(Vec<[u32; 4]>),
    Vec4F32(Vec<[f32; 4]>),
    Mat2U8(Vec<[[u8; 2]; 2]>),
    Mat2I8(Vec<[[i8; 2]; 2]>),
    Mat2U16(Vec<[[u16; 2]; 2]>),
    Mat2I16(Vec<[[i16; 2]; 2]>),
    Mat2U32(Vec<[[u32; 2]; 2]>),
    Mat2F32(Vec<[[f32; 2]; 2]>),
    Mat3U8(Vec<[[u8; 3]; 3]>),
    Mat3I8(Vec<[[i8; 3]; 3]>),
    Mat3U16(Vec<[[u16; 3]; 3]>),
    Mat3I16(Vec<[[i16; 3]; 3]>),
    Mat3U32(Vec<[[u32; 3]; 3]>),
    Mat3F32(Vec<[[f32; 3]; 3]>),
    Mat4U8(Vec<[[u8; 4]; 4]>),
    Mat4I8(Vec<[[i8; 4]; 4]>),
    Mat4U16(Vec<[[u16; 4]; 4]>),
    Mat4I16(Vec<[[i16; 4]; 4]>),
    Mat4U32(Vec<[[u32; 4]; 4]>),
    Mat4F32(Vec<[[f32; 4]; 4]>),
}
