use std::{borrow::Cow, collections::HashMap};

use awsm_renderer_core::{alignment::padding_for, buffer::{BufferDescriptor, BufferUsage}};

use crate::{
    buffers::helpers::{
        slice_zeroes, u8_to_f32_vec, u8_to_i16_vec, u8_to_i8_vec, u8_to_u16_vec, u8_to_u32_vec,
    },
    mesh::{
        MeshAttributeSemantic, MeshBufferIndexInfo, MeshBufferInfo, MeshBufferMorphInfo,
        MeshBufferVertexInfo,
    },
    AwsmRenderer,
};

use super::{
    accessors::semantic_ordering,
    error::{AwsmGltfError, Result},
};

#[derive(Debug)]
pub struct GltfBuffers {
    pub raw: Vec<Vec<u8>>,
    // this is definitely its own buffer
    // isn't passed to the shader at all
    pub index_bytes: Option<Vec<u8>>,
    pub index_buffer: Option<web_sys::GpuBuffer>,
    // this might later be split into positions, texcoords, normals, etc
    // but for now, we just want to pack it all into one buffer
    //
    // it's pretty common to treat positions as its own buffer, but, let's see...
    //
    // the important thing is that they always follow the same interleaving pattern
    // and we track where each primitive starts
    pub vertex_bytes: Vec<u8>,
    pub vertex_buffer: web_sys::GpuBuffer,

    // these also always follow the same interleaving pattern
    // and we track where each primitive starts
    pub morph_bytes: Option<Vec<u8>>,
    pub morph_buffer: Option<web_sys::GpuBuffer>,

    // first level is mesh, second level is primitive
    pub meshes: Vec<Vec<MeshBufferInfo>>,
}

impl GltfBuffers {
    pub async fn new(
        renderer: &AwsmRenderer,
        doc: &gltf::Document,
        buffers: Vec<Vec<u8>>,
    ) -> Result<Self> {
        // refactor original buffers into the format we want
        // namely, pack the data in a predictable order
        // arranged by primitive
        // with indices as a separate buffer

        let mut index_bytes: Vec<u8> = Vec::new();
        let mut vertex_bytes: Vec<u8> = Vec::new();
        let mut morph_bytes: Vec<u8> = Vec::new();
        let mut meshes: Vec<Vec<MeshBufferInfo>> = Vec::new();

        for mesh in doc.meshes() {
            let mut primitive_buffer_infos = Vec::new();

            for primitive in mesh.primitives() {
                // Index buffer
                let index = match primitive.indices() {
                    None => None,
                    Some(accessor) => {
                        let offset = index_bytes.len();
                        let accessor_bytes = accessor_to_bytes(&accessor, &buffers)?;
                        index_bytes.extend_from_slice(&accessor_bytes);

                        Some(MeshBufferIndexInfo {
                            offset,
                            count: accessor.count(),
                            stride: accessor.size(),
                        })
                    }
                };

                // Vertex buffer
                let vertex = {
                    let offset = vertex_bytes.len();

                    let mut attributes: Vec<(gltf::Semantic, gltf::Accessor<'_>)> =
                        primitive.attributes().collect();

                    attributes
                        .sort_by(|(a, _), (b, _)| semantic_ordering(a).cmp(&semantic_ordering(b)));

                    let mut attribute_stride_sizes = HashMap::new();
                    let mut attributes_bytes = Vec::new();

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
                    for (semantic, accessor) in attributes {
                        let semantic: MeshAttributeSemantic = semantic.into();
                        let attribute_bytes = accessor_to_bytes(&accessor, &buffers)?;

                        // while we're at it, we can stash the stride sizes
                        let attribute_stride_size = accessor
                            .view()
                            .and_then(|view| view.stride())
                            .unwrap_or(accessor.size());
                        attribute_stride_sizes.insert(semantic.clone(), attribute_stride_size);

                        attributes_bytes.push((attribute_bytes, attribute_stride_size));
                    }

                    // now let's predictably interleave the attributes into our final vertex buffer
                    // this does extend/copy the data, but it saves us additional calls at render time
                    for vertex in 0..vertex_count {
                        for (attribute_bytes, attribute_stride_size) in attributes_bytes.iter() {
                            let attribute_byte_offset = vertex * attribute_stride_size;
                            let attribute_bytes = &attribute_bytes[attribute_byte_offset
                                ..attribute_byte_offset + attribute_stride_size];

                            vertex_bytes.extend_from_slice(attribute_bytes);
                        }
                    }

                    MeshBufferVertexInfo {
                        offset,
                        count: vertex_count,
                        size: vertex_bytes.len() - offset,
                        attribute_stride_sizes,
                    }
                };

                // Morph buffer
                let morph = {
                    let morph_has_position = primitive
                        .morph_targets()
                        .any(|morph_target| morph_target.positions().is_some());
                    let morph_has_normal = primitive
                        .morph_targets()
                        .any(|morph_target| morph_target.normals().is_some());
                    let morph_has_tangent = primitive
                        .morph_targets()
                        .any(|morph_target| morph_target.tangents().is_some());

                    if !morph_has_position && !morph_has_normal && !morph_has_tangent {
                        None
                    } else {
                        let mut morph_targets_buffer_data = Vec::new();

                        #[derive(Default)]
                        struct MorphTargetBufferData<'a> {
                            positions: Option<Cow<'a, [u8]>>,
                            normals: Option<Cow<'a, [u8]>>,
                            tangents: Option<Cow<'a, [u8]>>,
                        }
                        for morph_target in primitive.morph_targets() {
                            let mut morph_target_buffer_data = MorphTargetBufferData::default();

                            if let Some(accessor) = morph_target.positions() {
                                morph_target_buffer_data.positions =
                                    Some(accessor_to_bytes(&accessor, &buffers)?);
                            }

                            if let Some(accessor) = morph_target.normals() {
                                morph_target_buffer_data.normals =
                                    Some(accessor_to_bytes(&accessor, &buffers)?);
                            }
                            if let Some(accessor) = morph_target.tangents() {
                                morph_target_buffer_data.tangents =
                                    Some(accessor_to_bytes(&accessor, &buffers)?);
                            }

                            morph_targets_buffer_data.push(morph_target_buffer_data);
                        }

                        // same idea as what we did with the vertex attributes
                        // but here we lay them out interleaved by morph target
                        // for example, the sequence would be:
                        // vertex 1, target 1: position, normal, tangent
                        // vertex 1, target 2: position, normal, tangent
                        // vertex 2, target 1: position, normal, tangent
                        // vertex 2, target 2: position, normal, tangent
                        //
                        // and then in the shader, for each vertex,
                        // it can read all the morph targets for that vertex
                        // essentially by just reading from its offset start to finish
                        //
                        // if a semantic is not used, we skip it instead of
                        // filling with 0's, since the shader will be different anyway

                        let offset = morph_bytes.len();

                        let mut vertex_morph_stride_size = 0;

                        for vertex_index in 0..vertex.count {
                            // eh, we could only set this once, but this is slightly nicer to read
                            // when the loop breaks we return the latest-and-greatest value
                            vertex_morph_stride_size = 0;

                            for morph_target_buffer_data in &morph_targets_buffer_data {
                                let mut push_bytes =
                                    |data: Option<&Cow<'_, [u8]>>, stride_size: usize| {
                                        match data {
                                            Some(data) => {
                                                let data_byte_offset = vertex_index * stride_size;
                                                let data_bytes = &data[data_byte_offset
                                                    ..data_byte_offset + stride_size];
                                                morph_bytes.extend_from_slice(data_bytes);
                                            }
                                            None => {
                                                morph_bytes
                                                    .extend_from_slice(slice_zeroes(stride_size));
                                            }
                                        }

                                        vertex_morph_stride_size += stride_size;
                                    };

                                if morph_has_position {
                                    let attribute_stride_size = *vertex
                                        .attribute_stride_sizes
                                        .get(&MeshAttributeSemantic::Position)
                                        .unwrap();
                                    push_bytes(
                                        morph_target_buffer_data.positions.as_ref(),
                                        attribute_stride_size,
                                    );
                                }

                                if morph_has_normal {
                                    let attribute_stride_size = *vertex
                                        .attribute_stride_sizes
                                        .get(&MeshAttributeSemantic::Normal)
                                        .unwrap();
                                    push_bytes(
                                        morph_target_buffer_data.normals.as_ref(),
                                        attribute_stride_size,
                                    );
                                }

                                if morph_has_tangent {
                                    let attribute_stride_size = *vertex
                                        .attribute_stride_sizes
                                        .get(&MeshAttributeSemantic::Tangent)
                                        .unwrap();
                                    push_bytes(
                                        morph_target_buffer_data.tangents.as_ref(),
                                        attribute_stride_size,
                                    );
                                }
                            }
                        }

                        let size = morph_bytes.len() - offset;
                        // pad to satisfy WebGPU
                        let padding = padding_for(size, 4);
                        if padding != 4 {
                            morph_bytes.extend_from_slice(slice_zeroes(padding));
                        }
                        Some(MeshBufferMorphInfo {
                            size: morph_bytes.len() - offset,
                            offset,
                            targets_len: primitive.morph_targets().len(),
                            vertex_stride_size: vertex_morph_stride_size,
                        })
                    }
                };

                // Done for this primitive
                primitive_buffer_infos.push(MeshBufferInfo {
                    index,
                    vertex,
                    morph,
                });
            }

            meshes.push(primitive_buffer_infos);
        }

        let index_buffer = match index_bytes.is_empty() {
            true => None,
            false => {
                // pad to multiple of 4 to satisfy WebGPU
                let padding = padding_for(index_bytes.len(), 4);
                if padding != 4 {
                    index_bytes.extend_from_slice(slice_zeroes(padding));
                }

                let index_buffer = renderer
                    .gpu
                    .create_buffer(
                        &BufferDescriptor::new(
                            Some("gltf index buffer"),
                            index_bytes.len(),
                            BufferUsage::new().with_copy_dst().with_index(),
                        )
                        .into(),
                    )
                    .map_err(AwsmGltfError::BufferCreate)?;

                renderer
                    .gpu
                    .write_buffer(&index_buffer, None, index_bytes.as_slice(), None, None)
                    .map_err(AwsmGltfError::BufferWrite)?;

                Some(index_buffer)
            }
        };

        let morph_buffer = match morph_bytes.is_empty() {
            true => None,
            false => {
                let morph_buffer = renderer
                    .gpu
                    .create_buffer(
                        &BufferDescriptor::new(
                            Some("gltf morph buffer"),
                            morph_bytes.len(),
                            BufferUsage::new().with_copy_dst().with_storage(),
                        )
                        .into(),
                    )
                    .map_err(AwsmGltfError::BufferCreate)?;

                renderer
                    .gpu
                    .write_buffer(&morph_buffer, None, morph_bytes.as_slice(), None, None)
                    .map_err(AwsmGltfError::BufferWrite)?;

                Some(morph_buffer)
            }
        };

        // pad to multiple of 4 to satisfy WebGPU
        let padding = padding_for(vertex_bytes.len(), 4);
        if padding != 4 {
            vertex_bytes.extend_from_slice(slice_zeroes(padding));
        }

        let vertex_buffer = renderer
            .gpu
            .create_buffer(
                &BufferDescriptor::new(
                    Some("gltf vertex buffer"),
                    vertex_bytes.len(),
                    BufferUsage::new().with_copy_dst().with_vertex(),
                )
                .into(),
            )
            .map_err(AwsmGltfError::BufferCreate)?;

        renderer
            .gpu
            .write_buffer(&vertex_buffer, None, vertex_bytes.as_slice(), None, None)
            .map_err(AwsmGltfError::BufferWrite)?;

        Ok(Self {
            raw: buffers,
            index_bytes: if index_bytes.is_empty() {
                None
            } else {
                Some(index_bytes)
            },
            index_buffer,
            vertex_bytes,
            vertex_buffer,
            morph_bytes: if morph_bytes.is_empty() {
                None
            } else {
                Some(morph_bytes)
            },
            morph_buffer,
            meshes,
        })
    }
}

impl Drop for GltfBuffers {
    fn drop(&mut self) {
        if let Some(index_buffer) = &self.index_buffer {
            index_buffer.destroy();
        }
        self.vertex_buffer.destroy();
    }
}

pub(super) fn accessor_to_bytes<'a>(
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
pub(super) fn accessor_to_vec(
    accessor: &gltf::Accessor<'_>,
    buffers: &[Vec<u8>],
) -> Result<AccessorVec> {
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
