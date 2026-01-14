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
    viewport: vec4<f32>, // in pixels, x,y,width,height
    dof_params: vec4<f32>, // x=focus_distance, y=aperture (f-stop), zw=unused
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
    viewport_pos: vec2<f32>, // x,y
    viewport_size: vec2<f32>, // width,height
    focus_distance: f32, // DoF focus distance in world units
    aperture: f32, // DoF aperture f-stop (lower = more blur)
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
    camera.viewport_pos = vec2<f32>(raw.viewport.x, raw.viewport.y);
    camera.viewport_size = vec2<f32>(raw.viewport.z, raw.viewport.w);
    camera.focus_distance = raw.dof_params.x;
    camera.aperture = raw.dof_params.y;
    return camera;
}
