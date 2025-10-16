struct StandardCoordinates {
    pixel_center: vec2<f32>,
    depth_sample: f32,
    ndc: vec3<f32>,
    clip_position: vec4<f32>,
    world_position: vec3<f32>,
    surface_to_camera: vec3<f32>,
}

fn get_standard_coordinates(coords: vec2<i32>, screen_dims: vec2<u32>) -> StandardCoordinates {
    let screen_dims_f32 = vec2<f32>(f32(screen_dims.x), f32(screen_dims.y));
    let depth_sample : f32 = textureLoad(depth_tex, coords, 0);

    // Pixel center in UV
    let uv = (vec2<f32>(vec2<i32>(coords) + vec2<i32>(1, 1)) - vec2<f32>(0.5, 0.5)) / screen_dims_f32;
    // Build NDC: flip Y because texture origin is top-left, NDC +Y is up.
    let ndc_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);

    // WebGPU/D3D/Vulkan convention: z already in [0,1]
    let ndc = vec3<f32>(ndc_xy, depth_sample);

    let clip_position = vec4<f32>(ndc, 1.0);
    let view_h        = camera.inv_proj * clip_position;
    let view_position = view_h.xyz / view_h.w;

    // inv_view is affine; resulting w will be 1, divide is unnecessary
    let world_position = (camera.inv_view * vec4<f32>(view_position, 1.0)).xyz;

    let to_camera = camera.position - world_position;
    let surface_to_camera = select(vec3<f32>(0.0, 0.0, 1.0),
                                   normalize(to_camera),
                                   length(to_camera) > 0.0);

    return StandardCoordinates(
        uv,               // or pixel_center if you prefer the name
        depth_sample,
        ndc,
        clip_position,
        world_position,
        surface_to_camera
    );
}
