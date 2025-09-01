{% include "geometry_wgsl/vertex/meta.wgsl" %}
{% include "geometry_wgsl/vertex/camera.wgsl" %}
{% include "geometry_wgsl/vertex/transform.wgsl" %}
{% include "geometry_wgsl/vertex/morph.wgsl" %}
{% include "geometry_wgsl/vertex/skin.wgsl" %}

//***** INPUT/OUTPUT *****
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,      // World position
    @location(1) triangle_id: u32,        // Triangle ID for this vertex
    @location(2) barycentric: vec2<f32>,   // Barycentric coordinates (x, y) - z = 1.0 - x - y
    {% if instancing_transforms %}
    // instance transform matrix
    @location(3) instance_transform_row_0: vec4<f32>,
    @location(4) instance_transform_row_1: vec4<f32>,
    @location(5) instance_transform_row_2: vec4<f32>,
    @location(6) instance_transform_row_3: vec4<f32>,
    {% endif %}
};

// Vertex output
struct VertexOutput {
    @builtin(position) screen_position: vec4<f32>,
    @location(0) world_position: vec3<f32>, 
    @location(1) clip_position: vec4<f32>,
    @location(2) @interpolate(flat) triangle_id: u32,
    @location(3) barycentric: vec3<f32>,  // Full barycentric coordinates
}

//***** MAIN *****
@vertex
fn vert_main(vertex_orig: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    var vertex = vertex_orig;

    if mesh_meta.morph_geometry_target_len != 0 {
        vertex = apply_position_morphs(vertex);
    }

    if mesh_meta.skin_sets_len != 0 {
        vertex = apply_position_skin(vertex);
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

    var pos = model_transform * vec4<f32>(vertex.position, 1.0);
    out.world_position = pos.xyz;
    out.clip_position = camera.view_proj * pos;
    out.screen_position = camera.view_proj * pos;
    
    // Pass through triangle ID
    out.triangle_id = vertex.triangle_id;
    
    // Reconstruct full barycentric coordinates
    // Input has (x, y), we calculate z = 1.0 - x - y
    out.barycentric = vec3<f32>(
        vertex.barycentric.x,
        vertex.barycentric.y,
        1.0 - vertex.barycentric.x - vertex.barycentric.y
    );
    
    return out;
}