// Raw camera uniform structure (matches GPU buffer layout with padding)
struct CameraRaw {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    position: vec4<f32>,  // .xyz = position, .w = unused
    frame_count_and_padding: vec4<u32>,  // .x = frame_count, .yzw = padding
    // IMPORTANT: frustum_rays are for SCREEN-SPACE RECONSTRUCTION, NOT frustum culling!
    // 4 normalized view-space ray directions at near plane corners [bottom-left, bottom-right, top-left, top-right]
    // Used for unprojecting screen pixels to world space with better precision than per-pixel unprojection
    frustum_rays: array<vec4<f32>, 4>,
    _padding_end: array<vec4<f32>, 2>,  // Total: 512 bytes
};

// Friendly camera structure (no padding, easier to work with)
struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    position: vec3<f32>,
    frame_count: u32,
    // Screen-space reconstruction rays (see CameraRaw for details)
    frustum_rays: array<vec4<f32>, 4>,
};

// Convert from raw uniform to friendly structure
fn camera_from_raw(raw: CameraRaw) -> Camera {
    var camera: Camera;
    camera.view = raw.view;
    camera.proj = raw.proj;
    camera.view_proj = raw.view_proj;
    camera.inv_view_proj = raw.inv_view_proj;
    camera.inv_proj = raw.inv_proj;
    camera.inv_view = raw.inv_view;
    camera.position = raw.position.xyz;
    camera.frame_count = raw.frame_count_and_padding.x;
    camera.frustum_rays = raw.frustum_rays;
    return camera;
}
