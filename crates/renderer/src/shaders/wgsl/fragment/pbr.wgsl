{% include "utils/color_space.wgsl" %}
{% include "fragment/lighting/math.wgsl" %}
{% include "fragment/lighting/lights.wgsl" %}
{% include "fragment/material/pbr_material.wgsl" %}
{% include "fragment/lighting/brdf.wgsl" %}
{% include "fragment/lighting/tonemap.wgsl" %}

/// Input from the vertex shader
struct FragmentInput {
    @builtin(position) screen_position: vec4<f32>, // for rasterization, interpolated
    @location(0) world_position: vec3<f32>, 
    @location(1) clip_position: vec4<f32>, // for motion vectors

    {% for loc in fragment_input_locations %}
        @location({{ loc.location }}) {{ loc.name }}: {{ loc.data_type }},
    {% endfor %}
};

{% for binding in fragment_buffer_bindings %}
    @group({{ binding.group }}) @binding({{ binding.index }}) var {{ binding.name }}: {{ binding.data_type }};
{% endfor %}

struct FragmentOutput {
    @location(0) scene: vec4<f32>,
    // texture target, used to calculate motion vectors
    @location(1) clip_position: vec4<f32>,
};

@fragment
fn frag_main(input: FragmentInput) -> FragmentOutput {
    var material = getMaterial(input);
    let n_lights = arrayLength(&lights) / 16u;

    {% if has_normals %}
        let normal = normalize(input.world_normal);
    {% else %}
        let normal = vec3<f32>(1.0, 1.0, 1.0);
    {% endif %}

    let surface_to_camera = normalize(camera.position - input.world_position);

    let ambient = vec3<f32>(0.1); // TODO - make this settable, or get from IBL
    var color = vec3<f32>(0.0);


    for(var i = 0u; i < n_lights; i = i + 1u) {
        let light_brdf = light_to_brdf(get_light(i), normal, input.world_position);

        if (light_brdf.n_dot_l > 0.0001) {
            color += brdf(input, material, light_brdf, ambient, surface_to_camera); 
        } else {
            color += ambient * material.base_color.rgb;
        }
    }

    var output: FragmentOutput;

    output.scene = vec4<f32>(color, material.base_color.a);
    output.clip_position = input.clip_position;

    return output;
}
