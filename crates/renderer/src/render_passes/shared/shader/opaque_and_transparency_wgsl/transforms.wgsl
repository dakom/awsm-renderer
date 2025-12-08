
struct Transforms {
    world_model: mat4x4<f32>,
    world_normal: mat3x3<f32>,
}

fn get_transforms(material_mesh_meta: MaterialMeshMeta) -> Transforms {
    let world_model = model_transforms[material_mesh_meta.transform_offset / 64u]; // 64 bytes per mat4x4<f32>

    let normal_matrix_offset = material_mesh_meta.normal_matrix_offset / 4u; // 4 bytes per float
    let world_normal = mat3x3<f32>(
        vec3<f32>(
            normal_matrices[normal_matrix_offset + 0u],
            normal_matrices[normal_matrix_offset + 1u],
            normal_matrices[normal_matrix_offset + 2u],
        ),
        vec3<f32>(
            normal_matrices[normal_matrix_offset + 3u],
            normal_matrices[normal_matrix_offset + 4u],
            normal_matrices[normal_matrix_offset + 5u],
        ),
        vec3<f32>(
            normal_matrices[normal_matrix_offset + 6u],
            normal_matrices[normal_matrix_offset + 7u],
            normal_matrices[normal_matrix_offset + 8u],
        ),
    );

    return Transforms(world_model, world_normal);
}
