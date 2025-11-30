use awsm_renderer_core::pipeline::vertex::{VertexAttribute, VertexBufferLayout, VertexFormat};

use crate::mesh::{
    Mesh, MeshBufferCustomVertexAttributeInfo, MeshBufferInfo, MeshBufferVertexAttributeInfo,
};

pub fn vertex_buffer_layout(mesh: &Mesh, buffer_info: &MeshBufferInfo) -> VertexBufferLayout {
    let mut shader_location = match mesh.instanced {
        // from VERTEX_BUFFER_LAYOUT attributes count
        true => 7,
        false => 3,
    } as u32;

    let mut attributes = vec![];

    let mut offset = 0;

    for attribute_info in buffer_info
        .triangles
        .vertex_attributes
        .iter()
        .filter(|x| x.is_custom_attribute())
    {
        let custom_attribute_info = match attribute_info {
            MeshBufferVertexAttributeInfo::Custom(info) => info,
            _ => unreachable!("Expected custom attribute info"),
        };

        attributes.push(VertexAttribute {
            format: custom_attribute_info.vertex_format(),
            offset,
            shader_location,
        });

        shader_location += 1;

        offset += attribute_info.vertex_size() as u64;
    }

    VertexBufferLayout {
        array_stride: offset,
        step_mode: None,
        attributes,
    }
}
