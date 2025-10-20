fn sample_skybox(
    coords: vec2<i32>,
    screen_dims: vec2<f32>,
    camera: CameraUniform,
    skybox_tex: texture_cube<f32>,
    skybox_sampler: sampler
) -> vec4<f32> {
    // Convert pixel coordinates to normalized device coordinates [0, 1]
    let uv = (vec2<f32>(coords) + vec2<f32>(0.5, 0.5)) / screen_dims;

    // Convert to clip space [-1, 1], with Y flipped for standard NDC
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);

    // Create a point in clip space at far plane
    let clip_pos = vec4<f32>(ndc.x, ndc.y, 1.0, 1.0);

    // Transform to view space using inverse projection
    let view_pos = camera.inv_proj * clip_pos;
    let view_dir = normalize(view_pos.xyz / view_pos.w);

    // Transform view direction to world space using inverse view matrix (rotation only)
    // Extract rotation from inverse view matrix by taking upper 3x3
    let inv_view_rotation = mat3x3<f32>(
        camera.inv_view[0].xyz,
        camera.inv_view[1].xyz,
        camera.inv_view[2].xyz
    );
    let ray_dir = inv_view_rotation * view_dir;

    // Sample the cubemap using the ray direction
    // Use textureSampleLevel for compute shaders (textureSample requires fragment stage)
    let color = textureSampleLevel(skybox_tex, skybox_sampler, ray_dir, 0.0);

    // Return raw HDR values - tone mapping happens in the display pass
    // The color space issue (appearing too dark) will be addressed separately
    return color;
}
