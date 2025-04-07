use std::borrow::Cow;

use awsm_renderer_core::buffer::{BufferDescriptor, BufferUsage};

use crate::AwsmRenderer;

use super::{
    accessors::semantic_ordering,
    error::{AwsmGltfError, Result},
};

#[derive(Debug)]
pub struct GltfBuffers {
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

    // first level is mesh, second level is primitive
    pub meshes: Vec<Vec<PrimitiveBufferInfo>>,
}

#[derive(Default, Debug, Clone)]
pub struct PrimitiveBufferInfo {
    pub index_offset: Option<usize>,
    pub index_len: Option<usize>,
    pub vertex_offset: usize,
    pub vertex_count: usize,
    pub vertex_strides: Vec<usize>,
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
        let mut meshes: Vec<Vec<PrimitiveBufferInfo>> = Vec::new();

        for mesh in doc.meshes() {
            let mut primitive_buffer_infos = Vec::new();

            for primitive in mesh.primitives() {
                // Write to index buffer
                let index_offset = match primitive.indices() {
                    None => None,
                    Some(accessor) => {
                        let index = index_bytes.len();
                        let other = accessor_to_bytes(&accessor, &buffers)?;
                        index_bytes.extend_from_slice(&other);
                        Some(index)
                    }
                };

                // Write to vertex buffer
                let vertex_offset = vertex_bytes.len();
                let mut attributes: Vec<(gltf::Semantic, gltf::Accessor<'_>)> =
                    primitive.attributes().collect();

                attributes
                    .sort_by(|(a, _), (b, _)| semantic_ordering(a).cmp(&semantic_ordering(b)));

                let mut vertex_strides = Vec::new();
                let mut attributes_bytes = Vec::new();

                // this should never be empty, but let's be safe
                let vertex_count = attributes
                    .iter()
                    .next()
                    .map(|(_, accessor)| accessor.count())
                    .unwrap_or(0);

                // first we need to read the whole accessor. This will be zero-copy unless one of these is true:
                // 1. they're sparse and we need to replace values
                // 2. there's no view, and we need to fill it with zeroes
                // 
                // otherwise, it's just a slice of the original buffer
                for (_, accessor) in attributes {
                    let attribute_bytes = accessor_to_bytes(&accessor, &buffers)?;

                    // while we're at it, we can stash the stride sizes
                    match accessor.view() {
                        Some(view) => {
                            vertex_strides.push(view.stride().unwrap_or(accessor.size()));
                        }
                        None => {
                            vertex_strides.push(accessor.size());
                        }
                    }

                    attributes_bytes.push(attribute_bytes);
                }

                // now let's predictably interleave the attributes into our final vertex buffer
                // this does extend/copy the data, but it saves us additional calls at render time
                for vertex in 0..vertex_count {
                    for attribute_index in 0..attributes_bytes.len() {
                        let vertex_stride = vertex_strides[attribute_index];
                        let attribute_byte_offset = vertex * vertex_stride;
                        let attribute_bytes = &attributes_bytes[attribute_index];
                        let attribute_bytes = &attribute_bytes[attribute_byte_offset..attribute_byte_offset + vertex_stride];

                        vertex_bytes.extend_from_slice(attribute_bytes);
                    }
                }

                // Done for this primitive
                primitive_buffer_infos.push(PrimitiveBufferInfo {
                    index_offset,
                    index_len: index_offset.map(|offset| index_bytes.len() - offset),
                    vertex_offset,
                    vertex_count,
                    vertex_strides,
                });
            }

            meshes.push(primitive_buffer_infos);
        }

        let index_buffer = match index_bytes.is_empty() {
            true => None,
            false => {
                // pad to multiple of 4 to satisfy WebGPU
                let pad = 4 - (index_bytes.len() % 4);
                if pad != 4 {
                    index_bytes.extend(vec![0; pad]);
                }

                let index_buffer = renderer
                    .gpu
                    .create_buffer(
                        &BufferDescriptor::new(
                            Some("gltf index buffer"),
                            index_bytes.len() as u64,
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

        // pad to multiple of 4 to satisfy WebGPU
        let pad = 4 - (vertex_bytes.len() % 4);
        if pad != 4 {
            vertex_bytes.extend(vec![0; pad]);
        }

        let vertex_buffer = renderer
            .gpu
            .create_buffer(
                &BufferDescriptor::new(
                    Some("gltf vertex buffer"),
                    vertex_bytes.len() as u64,
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
            index_bytes: if index_bytes.is_empty() {
                None
            } else {
                Some(index_bytes)
            },
            index_buffer,
            vertex_bytes,
            vertex_buffer,
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

fn accessor_to_bytes<'a>(
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

        tracing::info!("indices: {:?}", indices);

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

            // interpret the value_slice as a f32 using rust std
            tracing::info!(
                "from values: {}, {}, {} to {}, {}, {}",
                f32::from_le_bytes(buffer_slice[0..4].try_into().unwrap()),
                f32::from_le_bytes(buffer_slice[4..8].try_into().unwrap()),
                f32::from_le_bytes(buffer_slice[8..12].try_into().unwrap()),
                f32::from_le_bytes(value_slice[0..4].try_into().unwrap()),
                f32::from_le_bytes(value_slice[4..8].try_into().unwrap()),
                f32::from_le_bytes(value_slice[8..12].try_into().unwrap())
            );

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

#[allow(dead_code)]
pub(crate) fn debug_chunks_to_f32(slice: &[u8], chunk_size: usize) -> Vec<Vec<f32>> {
    debug_slice_to_f32(slice)
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

#[allow(dead_code)]
pub(crate) fn debug_slice_to_f32(slice: &[u8]) -> Vec<f32> {
    let mut f32s = Vec::new();
    for i in (0..slice.len()).step_by(4) {
        let bytes = &slice[i..i + 4];
        let f32_value = f32::from_le_bytes(bytes.try_into().unwrap());
        f32s.push(f32_value);
    }
    f32s
}

#[allow(dead_code)]
pub(crate) fn debug_slice_to_u16(slice: &[u8]) -> Vec<u16> {
    let mut u16s = Vec::new();
    for i in (0..slice.len()).step_by(2) {
        let bytes = &slice[i..i + 2];
        let u16_value = u16::from_le_bytes(bytes.try_into().unwrap());
        u16s.push(u16_value);
    }
    u16s
}

#[allow(dead_code)]
pub(crate) fn debug_slice_to_u32(slice: &[u8]) -> Vec<u32> {
    let mut u32s = Vec::new();
    for i in (0..slice.len()).step_by(4) {
        let bytes = &slice[i..i + 4];
        let u32_value = u32::from_le_bytes(bytes.try_into().unwrap());
        u32s.push(u32_value);
    }
    u32s
}
