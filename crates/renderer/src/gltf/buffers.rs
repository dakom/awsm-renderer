use std::borrow::Cow;

use awsm_renderer_core::buffer::{BufferDescriptor, BufferUsage};

use crate::AwsmRenderer;

use super::{accessors::semantic_ordering, error::{AwsmGltfError, Result}};

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
    pub meshes: Vec<Vec<MeshPrimitiveOffset>>
}

#[derive(Default, Debug, Clone)]
pub struct MeshPrimitiveOffset {
    pub index: Option<usize>,
    pub index_len: Option<usize>,
    pub vertex: usize,
    pub vertex_lens: Vec<usize>,
    pub vertex_strides: Vec<usize>
}

impl MeshPrimitiveOffset {
    pub fn total_vertex_stride(&self) -> usize {
        self.vertex_strides.iter().sum()
    }

    pub fn total_vertex_len(&self) -> usize {
        self.vertex_lens.iter().sum()
    }
}

impl GltfBuffers {
    pub async fn new(renderer: &AwsmRenderer, doc: &gltf::Document, buffers: Vec<Vec<u8>>) -> Result<Self> {
        // refactor original buffers into the format we want
        // namely, pack the data in a predictable order
        // arranged by primitive
        // with indices as a separate buffer

        let mut index_bytes:Vec<u8> = Vec::new();
        let mut vertex_bytes:Vec<u8> = Vec::new();
        let mut meshes:Vec<Vec<MeshPrimitiveOffset>> = Vec::new();

        for mesh in doc.meshes() {
            let mut primitive_offsets = Vec::new();

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
                let mut attributes:Vec<(gltf::Semantic, gltf::Accessor<'_>)> = primitive.attributes().collect();

                attributes.sort_by(|(a, _), (b, _)| {
                    semantic_ordering(a).cmp(&semantic_ordering(b))
                });

                let mut vertex_strides = Vec::new();
                let mut vertex_lens = Vec::new();

                for (_, accessor) in attributes {
                    let other = accessor_to_bytes(&accessor, &buffers)?;
                    vertex_bytes.extend_from_slice(&other);
                    vertex_lens.push(other.len());

                    match accessor.view() {
                        Some(view) => {
                            vertex_strides.push(view.stride().unwrap_or(accessor.size()));
                        },
                        None => {
                            vertex_strides.push(accessor.size());
                        }
                    }
                }

                // Done for this primitive
                primitive_offsets.push(MeshPrimitiveOffset {
                    index: index_offset,
                    index_len: match index_offset {
                        None => None,
                        Some(offset) => Some(index_bytes.len() - offset),
                    },
                    vertex: vertex_offset,
                    vertex_lens,
                    vertex_strides,
                });
            }

            meshes.push(primitive_offsets);
        }


        let index_buffer = match index_bytes.is_empty() {
            true => None,
            false=> {
                // pad to multiple of 4 to satisfy WebGPU
                let pad = 4 - (index_bytes.len() % 4);
                if pad != 4 {
                    index_bytes.extend(vec![0; pad]);
                }

                let index_buffer = renderer.gpu.create_buffer(&BufferDescriptor::new(
                    Some("gltf index buffer"),
                    index_bytes.len() as u64,
                    BufferUsage::new()
                        .with_copy_dst()
                        .with_index()
                ).into())
                .map_err(AwsmGltfError::BufferCreate)?;

                renderer.gpu.write_buffer(&index_buffer, None, index_bytes.as_slice(), None, None).map_err(AwsmGltfError::BufferWrite)?;
                
                Some(index_buffer)
            }
        };

        // pad to multiple of 4 to satisfy WebGPU
        let pad = 4 - (vertex_bytes.len() % 4);
        if pad != 4 {
            vertex_bytes.extend(vec![0; pad]);
        }

        let vertex_buffer = renderer.gpu.create_buffer(&BufferDescriptor::new(
            Some("gltf vertex buffer"),
            vertex_bytes.len() as u64,
            BufferUsage::new()
                .with_copy_dst()
                .with_vertex()
        ).into())
        .map_err(AwsmGltfError::BufferCreate)?;

        renderer.gpu.write_buffer(&vertex_buffer, None, vertex_bytes.as_slice(), None, None).map_err(AwsmGltfError::BufferWrite)?;

        Ok(Self {
            index_bytes: if index_bytes.is_empty() {
                None
            } else {
                Some(index_bytes)
            },
            index_buffer,
            vertex_bytes,
            vertex_buffer,
            meshes
        })
    }
}

fn accessor_to_bytes<'a>(accessor: &gltf::Accessor<'_>, buffers: &'a Vec<Vec<u8>>) -> Result<Cow<'a, [u8]>> {

    let length = accessor.size() * accessor.count();

    let mut buffer:Cow<[u8]> = match accessor.view() {
        Some(view) => {
            let buffer = &buffers[view.buffer().index()];
            let start = accessor.offset() + view.offset();
            let end = start + length;
            Cow::Borrowed(&buffer[start..end])
        },
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
            let value_slice = &values_buffer_slice[value_slice_start..value_slice_start + accessor.size()];

            let buffer_slice_start = target_index * accessor.size();
            let buffer_slice = &mut buffer[buffer_slice_start..buffer_slice_start + accessor.size()];

            // interpret the value_slice as a f32 using rust std
            tracing::info!("from values: {}, {}, {} to {}, {}, {}", 
                f32::from_le_bytes(buffer_slice[0..4].try_into().unwrap()),
                f32::from_le_bytes(buffer_slice[4..8].try_into().unwrap()),
                f32::from_le_bytes(buffer_slice[8..12].try_into().unwrap()),
                f32::from_le_bytes(value_slice[0..4].try_into().unwrap()),
                f32::from_le_bytes(value_slice[4..8].try_into().unwrap()),
                f32::from_le_bytes(value_slice[8..12].try_into().unwrap())
            );

            buffer_slice.copy_from_slice(&value_slice);
        }
    }

    Ok(buffer)

} 

fn sparse_to_indices(sparse: &gltf::accessor::sparse::Sparse<'_>, buffers: &Vec<Vec<u8>>) -> Vec<usize> {
    let indices_buffer_slice = &buffers[sparse.indices().view().buffer().index()];
    let indices_buffer_slice_start = sparse.indices().offset() + sparse.indices().view().offset();
    let indices_buffer_slice = &indices_buffer_slice[indices_buffer_slice_start..];

    let mut index_offset = 0;
    let index_offset_amount = sparse.indices().index_type().size();

    let mut indices = Vec::with_capacity(sparse.count());

    for _ in 0..sparse.count() {
        let index = match sparse.indices().index_type() {
            gltf::accessor::sparse::IndexType::U8 => {
                let index = indices_buffer_slice[index_offset];
                index as usize
            }, 
            gltf::accessor::sparse::IndexType::U16 => {
                let index = indices_buffer_slice[index_offset..index_offset + 2].try_into().unwrap();
                u16::from_le_bytes(index) as usize
            },
            gltf::accessor::sparse::IndexType::U32 => {
                let index = indices_buffer_slice[index_offset..index_offset + 4].try_into().unwrap();
                u32::from_le_bytes(index) as usize
            }
        };
        indices.push(index);
        index_offset += index_offset_amount;
    }

    indices
}
