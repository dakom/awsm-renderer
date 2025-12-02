//***** MAIN *****
struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,      // Model-space position
    @location(1) normal: vec3<f32>,        // Model-space normal
    @location(2) tangent: vec4<f32>,       // Model-space tangent (w = handedness)
    {% if instancing_transforms %}
    // instance transform matrix
    @location(3) instance_transform_row_0: vec4<f32>,
    @location(4) instance_transform_row_1: vec4<f32>,
    @location(5) instance_transform_row_2: vec4<f32>,
    @location(6) instance_transform_row_3: vec4<f32>,
    {% endif %}

    {% for i in 0..color_sets %}
        @location({{ in_color_set_start + i }}) color_{{ i }}: vec2<f32>,
    {% endfor %}

    {% for i in 0..uv_sets %}
        @location({{ in_uv_set_start + i }}) uv_{{ i }}: vec2<f32>,
    {% endfor %}
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,     // Transformed world-space normal
    @location(1) world_tangent: vec4<f32>,    // Transformed world-space tangent (w = handedness)

    {% for i in 0..color_sets %}
        @location({{ out_color_set_start + i }}) color_{{ i }}: vec2<f32>,
    {% endfor %}

    {% for i in 0..uv_sets %}
        @location({{ out_uv_set_start + i }}) uv_{{ i }}: vec2<f32>,
    {% endfor %}
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

    {% for i in 0..color_sets %}
        out.color_{{ i }} = input.color_{{ i }};
    {% endfor %}

    {% for i in 0..uv_sets %}
        out.uv_{{ i }} = input.uv_{{ i }};
    {% endfor %}

    return out;
}
