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
    world_position: vec3<f32>,   // Transformed world-space position
}

fn apply_vertex(vertex_orig: ApplyVertexInput, camera: Camera) -> ApplyVertexOutput {
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

    let world_pos = model_transform * vec4<f32>(vertex.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;


    // Transform normal/tangent to world space (ignore translation)
    let model_matrix3 = mat3x3<f32>(
        model_transform[0].xyz,
        model_transform[1].xyz,
        model_transform[2].xyz
    );
    // Correct normal transform for non-uniform scaling using an explicit
    // inverse-transpose (cofactor) path, avoiding WGSL inverse() support issues.
    let c0 = model_matrix3[0];
    let c1 = model_matrix3[1];
    let c2 = model_matrix3[2];
    let r0 = vec3<f32>(c0.x, c1.x, c2.x);
    let r1 = vec3<f32>(c0.y, c1.y, c2.y);
    let r2 = vec3<f32>(c0.z, c1.z, c2.z);

    let cof0 = cross(r1, r2);
    let cof1 = cross(r2, r0);
    let cof2 = cross(r0, r1);
    let det_model = dot(r0, cof0);

    let world_normal_unnormalized = select(
        model_matrix3 * normal,
        vec3<f32>(
            dot(cof0, normal),
            dot(cof1, normal),
            dot(cof2, normal),
        ) / det_model,
        abs(det_model) > 1e-8
    );
    let world_normal = normalize(world_normal_unnormalized);

    // Tangents transform with the model matrix, then must be re-orthonormalized against N.
    let tangent_raw = model_matrix3 * tangent.xyz;
    var tangent_ortho = tangent_raw - world_normal * dot(tangent_raw, world_normal);
    let tangent_len_sq = dot(tangent_ortho, tangent_ortho);
    if (tangent_len_sq > 1e-8) {
        tangent_ortho *= inverseSqrt(tangent_len_sq);
    } else {
        // Deterministic fallback tangent orthogonal to N.
        let fallback_axis = select(
            vec3<f32>(0.0, 0.0, 1.0),
            vec3<f32>(0.0, 1.0, 0.0),
            abs(world_normal.z) > 0.999
        );
        tangent_ortho = normalize(cross(fallback_axis, world_normal));
    }

    out.world_normal = world_normal;
    out.world_tangent = vec4<f32>(tangent_ortho, tangent.w);

    out.world_position = world_pos.xyz;

    return out;
}
