struct StandardCoordinates {
    pixel_center: vec2<f32>,
    depth_sample: f32,
    ndc: vec3<f32>,
    clip_position: vec4<f32>,
    world_position: vec3<f32>,
    view_position: vec3<f32>,
    surface_to_camera: vec3<f32>,
}

fn get_standard_coordinates(coords: vec2<i32>, screen_dims: vec2<u32>) -> StandardCoordinates {
    let screen_dims_f32 = vec2<f32>(f32(screen_dims.x), f32(screen_dims.y));
    let depth_sample : f32 = textureLoad(depth_tex, coords, 0);

    // Pixel center UV and NDC (flip Y once)
    let uv = (vec2<f32>(coords) + 0.5) / screen_dims_f32;
    let ndc_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);

    // WebGPU: NDC.z in [0,1]; no remap
    let ndc = vec3<f32>(ndc_xy, depth_sample);
    let clip_position = vec4<f32>(ndc, 1.0);

    let view_h        = camera.inv_proj * clip_position;
    let view_position = view_h.xyz / max(view_h.w, 1e-8);

    let world_position = (camera.inv_view * vec4<f32>(view_position, 1.0)).xyz;

    let to_camera = camera.position - world_position;
    let surface_to_camera = select(vec3<f32>(0.0, 0.0, 1.0),
                                   safe_normalize(to_camera),
                                   dot(to_camera, to_camera) > 0.0);

    return StandardCoordinates(
        uv,
        depth_sample,
        ndc,
        clip_position,
        world_position,
        view_position,
        surface_to_camera
    );
}
