use awsm_renderer_core::pipeline::vertex::{
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use super::{buffers::GltfMeshBufferInfo, error::Result};

pub(super) fn primitive_vertex_buffer_layout(
    buffer_info: &GltfMeshBufferInfo,
) -> Result<(VertexBufferLayout, u32)> {
    let mut attributes = Vec::new();

    let mut shader_location: u32 = 0;

    let mut offset = 0u64;
    for attribute in &buffer_info.vertex.attributes {
        for _ in 0..attribute.shader_key_kind.count() {
            attributes.push(VertexAttribute {
                format: attribute.format,
                offset,
                shader_location,
            });

            offset += attribute.size as u64;
            shader_location += 1;
        }
    }

    Ok((
        VertexBufferLayout {
            // this is the stride across all of the attributes
            array_stride: offset,
            step_mode: None,
            attributes,
        },
        shader_location,
    ))
}

pub(super) fn instance_transform_vertex_buffer_layout(shader_location: u32) -> VertexBufferLayout {
    // one mat4 (4 × vec4 x f32) per instance
    const STRIDE: u64 = 4 * 4 * 4;

    VertexBufferLayout {
        array_stride: STRIDE,
        step_mode: Some(VertexStepMode::Instance),
        attributes: vec![
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 4 * 4,
                shader_location: shader_location + 1,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 8 * 4,
                shader_location: shader_location + 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 12 * 4,
                shader_location: shader_location + 3,
            },
        ],
    }
}
