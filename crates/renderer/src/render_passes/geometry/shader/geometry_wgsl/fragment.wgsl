@group(1) @binding(1) var<uniform> material_header: MaterialHeader;

struct MaterialHeader {
    material_offset: u32,
};


struct FragmentInput {
    @builtin(position) screen_position: vec4<f32>,

    @location(0) world_position: vec3<f32>, 
    @location(1) clip_position: vec4<f32>,

    {% if has_normals %}
        @location(2) world_normal: vec3<f32>,
    {% endif %}
};

struct FragmentOutput {
    @location(0) material_offset: u32,
    @location(1) world_normal: vec4<f32>,
    @location(2) screen_pos: vec4<f32>,
    @location(3) motion_vector: vec2<f32>,
}

@fragment
fn frag_main(input: FragmentInput) -> FragmentOutput {
    var output:FragmentOutput;

    {% if has_normals %}
        output.world_normal = vec4<f32>(input.world_normal, 1.0);
    {% endif %}

    output.screen_pos = input.clip_position;
    //output.material_offset = material_header.material_offset;
    output.material_offset = 1;

    return output;
}
