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

    // Sample the depth buffer written by the visibility pass. Because we request level 0 the GPU
    // picks the highest-resolution mip. The resulting value is still in clip-space depth, so we
    // convert back to NDC and then to world space with the inverse view-projection matrix.
    let depth_sample = textureLoad(depth_tex, coords, 0);
    // Convert the integer pixel coordinate into normalized device coordinates using the pixel
    // centre. This matches how rasterisation computes attribute interpolation.
    let pixel_center = (vec2<f32>(f32(coords.x), f32(coords.y)) + vec2<f32>(0.5, 0.5)) / screen_dims_f32;
    let ndc = vec3<f32>(pixel_center * 2.0 - vec2<f32>(1.0, 1.0), depth_sample * 2.0 - 1.0);
    let clip_position = vec4<f32>(ndc, 1.0);
    let world_position_h = camera.inv_view_proj * clip_position;
    let world_position = world_position_h.xyz / world_position_h.w;
    let to_camera = camera.position - world_position;
    var surface_to_camera = vec3<f32>(0.0, 0.0, 1.0);
    if (length(to_camera) > 0.0) {
        surface_to_camera = normalize(to_camera);
    }

    return StandardCoordinates(
        pixel_center,
        depth_sample,
        ndc,
        clip_position,
        world_position,
        surface_to_camera
    );
}
