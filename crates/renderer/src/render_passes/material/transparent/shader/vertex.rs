use awsm_renderer_core::pipeline::vertex::VertexBufferLayout;

use crate::{
    mesh::Mesh,
    render_passes::shared::geometry_and_transparency::vertex::{
        VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY,
        VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY_INSTANCING,
    },
};

pub fn vertex_buffer_layout(mesh: &Mesh) -> VertexBufferLayout {
    let start_location = match mesh.instanced {
        true => {
            VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY
                .attributes
                .len()
                + VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY_INSTANCING
                    .attributes
                    .len()
        }
        false => VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY
            .attributes
            .len(),
    } as u32;

    VertexBufferLayout {
        // this is the stride across all of the attributes
        // TODO - probably need to calculate this dynamically
        array_stride: 0,
        step_mode: None,
        attributes: vec![],
    }
}

// pub static VERTEX_BUFFER_LAYOUT_GEOMETRY_AND_TRANSPARENCY: LazyLock<VertexBufferLayout> =
//     LazyLock::new(|| {
//         VertexBufferLayout {
//             // this is the stride across all of the attributes
//             // position (12) + triangle_index (4) + barycentric (8) + normal (12) + tangent (16) = 52 bytes
//             array_stride: MeshBufferVertexInfo::BYTE_SIZE as u64,
//             step_mode: None,
//             attributes: vec![
//                 // Position (vec3<f32>) at offset 0
//                 VertexAttribute {
//                     format: VertexFormat::Float32x3,
//                     offset: 0,
//                     shader_location: 0,
//                 },
//                 // Triangle ID (u32) at offset 12
//                 VertexAttribute {
//                     format: VertexFormat::Uint32,
//                     offset: 12,
//                     shader_location: 1,
//                 },
//                 // Barycentric coordinates (vec2<f32>) at offset 16
//                 VertexAttribute {
//                     format: VertexFormat::Float32x2,
//                     offset: 16,
//                     shader_location: 2,
//                 },
//                 // Normal (vec3<f32>) at offset 24
//                 VertexAttribute {
//                     format: VertexFormat::Float32x3,
//                     offset: 24,
//                     shader_location: 3,
//                 },
//                 // Tangent (vec4<f32>) at offset 36
//                 VertexAttribute {
//                     format: VertexFormat::Float32x4,
//                     offset: 36,
//                     shader_location: 4,
//                 },
//             ],
//         }
//     });
