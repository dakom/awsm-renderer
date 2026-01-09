// Fragment input from vertex shader
struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,     // World position
    @location(1) world_normal: vec3<f32>,     // Transformed world-space normal
    @location(2) world_tangent: vec4<f32>,    // Transformed world-space tangent (w = handedness)
    {% for i in 0..color_sets %}
        @location({{ in_color_set_start + i }}) color_{{ i }}: vec4<f32>,
    {% endfor %}

    {% for i in 0..uv_sets %}
        @location({{ in_uv_set_start + i }}) uv_{{ i }}: vec2<f32>,
    {% endfor %}
}

struct FragmentOutput {
    // Rgba16float
    @location(0) color: vec4<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;

    // Convert raw camera uniform to friendly structure
    let camera = camera_from_raw(camera_raw);

    // Get material from mesh metadata
    let material = pbr_get_material(material_mesh_meta.material_offset);

    // Sample all PBR material textures and compute material properties
    let material_color = pbr_get_material_color(
        material,
        input.world_normal,
        input.world_tangent,
        input
    );

    // Calculate surface to camera vector for lighting
    let surface_to_camera = normalize(camera.position - input.world_position);

    {% if !unlit %}
        // Get lighting info and apply all enabled lights
        let lights_info = get_lights_info();
        let color = apply_lighting(
            material_color,
            surface_to_camera,
            input.world_position,
            lights_info
        );
    {% else %}
        let color = unlit(material_color);
    {% endif %}

    // Output final color with alpha
    let premult_rgb = color * material_color.base.a;
    out.color = vec4<f32>(premult_rgb, material_color.base.a);

    return out;
}
