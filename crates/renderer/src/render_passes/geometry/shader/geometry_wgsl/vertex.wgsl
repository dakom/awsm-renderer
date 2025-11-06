{% include "geometry_wgsl/vertex/meta.wgsl" %}
{% include "geometry_wgsl/vertex/camera.wgsl" %}
{% include "geometry_wgsl/vertex/transform.wgsl" %}
{% include "geometry_wgsl/vertex/morph.wgsl" %}
{% include "geometry_wgsl/vertex/skin.wgsl" %}

//***** INPUT/OUTPUT *****
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,      // Model-space position
    @location(1) triangle_index: u32,      // Triangle index for this vertex
    @location(2) barycentric: vec2<f32>,   // Barycentric coordinates (x, y) - z = 1.0 - x - y
    @location(3) normal: vec3<f32>,        // Model-space normal
    @location(4) tangent: vec4<f32>,       // Model-space tangent (w = handedness)
    {% if instancing_transforms %}
    // instance transform matrix
    @location(5) instance_transform_row_0: vec4<f32>,
    @location(6) instance_transform_row_1: vec4<f32>,
    @location(7) instance_transform_row_2: vec4<f32>,
    @location(8) instance_transform_row_3: vec4<f32>,
    {% endif %}
};

// Vertex output
struct VertexOutput {
    @builtin(position) screen_position: vec4<f32>,
    // same value as screen_position
    @location(1) clip_position: vec4<f32>,
    @location(2) @interpolate(flat) triangle_index: u32,
    @location(3) barycentric: vec2<f32>,  // Full barycentric coordinates
    @location(4) world_normal: vec3<f32>,     // Transformed world-space normal
    @location(5) world_tangent: vec4<f32>,    // Transformed world-space tangent (w = handedness)
}

//***** MAIN *****
@vertex
fn vert_main(vertex_orig: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    var vertex = vertex_orig;
    var normal = vertex_orig.normal;
    var tangent = vertex_orig.tangent;

    // Apply morphs to position, normal, and tangent
    if mesh_meta.morph_geometry_target_len != 0 {
        vertex = apply_position_morphs(vertex);

        // Apply morphed normals (correct behavior)
        normal = apply_normal_morphs(vertex_orig, normal);
        tangent = apply_tangent_morphs(vertex_orig, tangent);
    }

    // Apply skinning to position, normal, and tangent
    if mesh_meta.skin_sets_len != 0 {
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

        let model_transform = get_model_transform(mesh_meta.transform_offset) * instance_transform;
    {% else %}
        let model_transform = get_model_transform(mesh_meta.transform_offset);
    {% endif %}

    let pos = model_transform * vec4<f32>(vertex.position, 1.0);
    out.clip_position = camera.view_proj * pos;
    out.screen_position = camera.view_proj * pos;

    // Pass through triangle index
    out.triangle_index = vertex.triangle_index;

    // Reconstruct full barycentric coordinates
    out.barycentric = vertex.barycentric;

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
