{% include "geometry_and_transparency_wgsl/vertex/meta.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/camera.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/transform.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/morph.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/skin.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/apply.wgsl" %}


//***** MAIN *****
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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) triangle_index: u32,
    @location(1) barycentric: vec2<f32>,  // Full barycentric coordinates
    @location(2) world_normal: vec3<f32>,     // Transformed world-space normal
    @location(3) world_tangent: vec4<f32>,    // Transformed world-space tangent (w = handedness)
}

@vertex
fn vert_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let applied = apply_vertex(ApplyVertexInput(
        input.vertex_index,
        input.position,
        input.normal,
        input.tangent,
        {% if instancing_transforms %}
            input.instance_transform_row_0,
            input.instance_transform_row_1,
            input.instance_transform_row_2,
            input.instance_transform_row_3,
        {% endif %}
    ));

    out.clip_position = applied.clip_position;
    out.world_normal = applied.world_normal;
    out.world_tangent = applied.world_tangent;

    // Pass through
    out.triangle_index = input.triangle_index;
    out.barycentric = input.barycentric;

    return out;
}
