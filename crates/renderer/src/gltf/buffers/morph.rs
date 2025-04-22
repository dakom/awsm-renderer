use std::borrow::Cow;

use gltf::Semantic;

use super::vertex::GltfMeshBufferVertexInfo;
use super::Result;
use crate::buffer::helpers::slice_zeroes;
use crate::gltf::buffers::accessor::accessor_to_bytes;
use crate::mesh::MeshBufferMorphInfo;

#[derive(Default, Debug, Clone)]
pub struct GltfMeshBufferMorphInfo {
    // offset in morph_bytes where this primitive starts
    pub values_offset: usize,

    // number of morph targets
    pub targets_len: usize,
    // the stride of all morph targets across the vertice, without padding
    pub vertex_stride_size: usize,
    // the size of the whole slice of data (all vertices and targets)
    pub values_size: usize,
}

impl From<GltfMeshBufferMorphInfo> for MeshBufferMorphInfo {
    fn from(info: GltfMeshBufferMorphInfo) -> Self {
        Self {
            targets_len: info.targets_len,
            vertex_stride_size: info.vertex_stride_size,
            values_size: info.values_size,
        }
    }
}

impl GltfMeshBufferMorphInfo {
    pub fn maybe_new(
        primitive: &gltf::Primitive<'_>,
        buffers: &[Vec<u8>],
        vertex_info: &GltfMeshBufferVertexInfo,
        morph_bytes: &mut Vec<u8>,
    ) -> Result<Option<Self>> {
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
            Ok(None)
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
                        Some(accessor_to_bytes(&accessor, buffers)?);
                }

                if let Some(accessor) = morph_target.normals() {
                    morph_target_buffer_data.normals = Some(accessor_to_bytes(&accessor, buffers)?);
                }
                if let Some(accessor) = morph_target.tangents() {
                    morph_target_buffer_data.tangents =
                        Some(accessor_to_bytes(&accessor, buffers)?);
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

            let values_offset = morph_bytes.len();

            let mut vertex_morph_stride_size = 0;

            for vertex_index in 0..vertex_info.count {
                // eh, we could only set this once, but this is slightly nicer to read
                // when the loop breaks we return the latest-and-greatest value
                vertex_morph_stride_size = 0;

                for morph_target_buffer_data in &morph_targets_buffer_data {
                    let mut push_bytes = |data: Option<&Cow<'_, [u8]>>, stride_size: usize| {
                        match data {
                            Some(data) => {
                                let data_byte_offset = vertex_index * stride_size;
                                let data_bytes =
                                    &data[data_byte_offset..data_byte_offset + stride_size];
                                morph_bytes.extend_from_slice(data_bytes);
                            }
                            None => {
                                morph_bytes.extend_from_slice(slice_zeroes(stride_size));
                            }
                        }

                        vertex_morph_stride_size += stride_size;
                    };

                    if morph_has_position {
                        let attribute_stride_size = *vertex_info
                            .attribute_stride_sizes
                            .get(&Semantic::Positions)
                            .unwrap();
                        push_bytes(
                            morph_target_buffer_data.positions.as_ref(),
                            attribute_stride_size,
                        );
                    }

                    if morph_has_normal {
                        let attribute_stride_size = *vertex_info
                            .attribute_stride_sizes
                            .get(&Semantic::Normals)
                            .unwrap();
                        push_bytes(
                            morph_target_buffer_data.normals.as_ref(),
                            attribute_stride_size,
                        );
                    }

                    if morph_has_tangent {
                        let attribute_stride_size = *vertex_info
                            .attribute_stride_sizes
                            .get(&Semantic::Tangents)
                            .unwrap();
                        push_bytes(
                            morph_target_buffer_data.tangents.as_ref(),
                            attribute_stride_size,
                        );
                    }
                }
            }

            Ok(Some(Self {
                values_offset,
                values_size: morph_bytes.len() - values_offset,
                targets_len: primitive.morph_targets().len(),
                vertex_stride_size: vertex_morph_stride_size,
            }))
        }
    }
}
