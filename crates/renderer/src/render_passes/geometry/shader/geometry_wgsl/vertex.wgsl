{% include "geometry_wgsl/vertex/morph.wgsl" %}
{% include "geometry_wgsl/vertex/skin.wgsl" %}

struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    position: vec3<f32>,
    frame_count: u32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> u_transform: TransformUniform;

struct TransformUniform {
    model: mat4x4<f32>,
};

@group(2) @binding(0)
var<uniform> mesh_meta: MeshMeta;

struct MeshMeta {
    mesh_key_high: u32,
    mesh_key_low: u32,
    material_offset: u32,
    morph_geometry_target_len: u32,
    morph_material_target_len: u32,
    morph_material_bitmask: u32,
    skin_sets_len: u32
}


//***** INPUT/OUTPUT *****
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,      // World position
    @location(1) triangle_id: u32,        // Triangle ID for this vertex
    @location(2) barycentric: vec2<f32>,   // Barycentric coordinates (x, y) - z = 1.0 - x - y
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


    let model_transform = u_transform.model;
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