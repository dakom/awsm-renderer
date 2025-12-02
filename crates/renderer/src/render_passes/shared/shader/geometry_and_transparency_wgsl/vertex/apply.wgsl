//***** INPUT/OUTPUT *****

struct ApplyVertexInput {
    vertex_index: u32,
    position: vec3<f32>,      // Model-space position
    normal: vec3<f32>,        // Model-space normal
    tangent: vec4<f32>,       // Model-space tangent (w = handedness)
    {% if instancing_transforms %}
        // instance transform matrix
        instance_transform_row_0: vec4<f32>,
        instance_transform_row_1: vec4<f32>,
        instance_transform_row_2: vec4<f32>,
        instance_transform_row_3: vec4<f32>,
    {% endif %}
}

struct ApplyVertexOutput {
    clip_position: vec4<f32>,
    world_normal: vec3<f32>,     // Transformed world-space normal
    world_tangent: vec4<f32>,    // Transformed world-space tangent (w = handedness)
}

fn apply_vertex(vertex_orig: ApplyVertexInput) -> ApplyVertexOutput {
    var out: ApplyVertexOutput;

    var vertex = vertex_orig;
    var normal = vertex_orig.normal;
    var tangent = vertex_orig.tangent;

    // Apply morphs to position, normal, and tangent
    if geometry_mesh_meta.morph_geometry_target_len != 0 {
        vertex = apply_position_morphs(vertex);

        // Apply morphed normals (correct behavior)
        normal = apply_normal_morphs(vertex_orig, normal);
        tangent = apply_tangent_morphs(vertex_orig, tangent);
    }

    // Apply skinning to position, normal, and tangent
    if geometry_mesh_meta.skin_sets_len != 0 {
        vertex = apply_position_skin(vertex);
        normal = apply_normal_skin(vertex_orig, normal);
        tangent = vec4<f32>(apply_normal_skin(vertex_orig, tangent.xyz), tangent.w);
    }

    {% if instancing_transforms %}
        // Transform the vertex position by the instance transform
        let instance_transform = mat4x4<f32>(
            vertex.instance_transform_row_0,
            vertex.instance_transform_row_1,
            vertex.instance_transform_row_2,
            vertex.instance_transform_row_3,
        );

        let model_transform = get_model_transform(geometry_mesh_meta.transform_offset) * instance_transform;
    {% else %}
        let model_transform = get_model_transform(geometry_mesh_meta.transform_offset);
    {% endif %}

    let pos = model_transform * vec4<f32>(vertex.position, 1.0);
    out.clip_position = camera.view_proj * pos;


    // Transform normal to world space (use mat3 to ignore translation)
    let normal_matrix = mat3x3<f32>(
        model_transform[0].xyz,
        model_transform[1].xyz,
        model_transform[2].xyz
    );
    out.world_normal = normalize(normal_matrix * normal);

    out.world_tangent = vec4<f32>(normalize(normal_matrix * tangent.xyz), tangent.w);

    return out;
}
