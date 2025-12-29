@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    position: vec3<f32>,
    frame_count: u32,
    frustum_rays: array<vec4<f32>, 4>,
};


@vertex
fn vert_main(@builtin(vertex_index) vertex_index: u32) -> FragmentInput {
    var out: FragmentInput;

    // Generate oversized triangle vertices using bit manipulation
    // Goal: vertex 0→(-1,-1), vertex 1→(3,-1), vertex 2→(-1,3)

    // X coordinate generation:
    // vertex_index: 0 → 0<<1 = 0 → 0&2 = 0 → 0*2-1 = -1 ✓
    // vertex_index: 1 → 1<<1 = 2 → 2&2 = 2 → 2*2-1 = 3  ✓
    // vertex_index: 2 → 2<<1 = 4 → 4&2 = 0 → 0*2-1 = -1 ✓
    let x = f32((vertex_index << 1u) & 2u) * 2.0 - 1.0;

    // Y coordinate generation:
    // vertex_index: 0 → 0&2 = 0 → 0*2-1 = -1 ✓
    // vertex_index: 1 → 1&2 = 0 → 0*2-1 = -1 ✓
    // vertex_index: 2 → 2&2 = 2 → 2*2-1 = 3  ✓
    let y = f32(vertex_index & 2u) * 2.0 - 1.0;

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.ndc = vec2<f32>(x, y);

    return out;
}

struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) ndc: vec2<f32>,
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

// ===== GRID CONFIGURATION =====
// Blender-like color scheme
const GRID_COLOR_BACKGROUND: vec3<f32> = vec3<f32>(0.18, 0.18, 0.18); // Light gray background/tiles
const GRID_COLOR_MINOR: vec3<f32> = vec3<f32>(0.28, 0.28, 0.28);      // Medium gray minor lines
const GRID_COLOR_MAJOR: vec3<f32> = vec3<f32>(0.38, 0.38, 0.38);      // Lighter gray major lines
const GRID_COLOR_X_AXIS: vec3<f32> = vec3<f32>(0.95, 0.3, 0.3);       // Red X axis
const GRID_COLOR_Z_AXIS: vec3<f32> = vec3<f32>(0.3, 0.5, 0.95);       // Blue Z axis

const GRID_ALPHA_BACKGROUND: f32 = 0.7;
const GRID_ALPHA_MINOR: f32 = 0.9;
const GRID_ALPHA_MAJOR: f32 = 1.0;
const GRID_ALPHA_AXIS: f32 = 1.0;

// Fade distances to reduce moire
const FADE_MINOR_START: f32 = 15.0;
const FADE_MINOR_END: f32 = 30.0;
const FADE_MAJOR_START: f32 = 80.0;
const FADE_MAJOR_END: f32 = 150.0;

@fragment
fn frag_main(in: FragmentInput) -> FragmentOutput {
    let ndc = in.ndc;

    // Extract camera basis from view matrix
    let view_right = normalize(vec3<f32>(camera.view[0].x, camera.view[1].x, camera.view[2].x));
    let view_up = normalize(vec3<f32>(camera.view[0].y, camera.view[1].y, camera.view[2].y));
    let view_forward = normalize(vec3<f32>(camera.view[0].z, camera.view[1].z, camera.view[2].z));

    // Construct ray direction (needs FOV scaling for proper perspective)
    let fov_scale = 1.0;
    let ray_dir = normalize(view_right * ndc.x * fov_scale + view_up * ndc.y * fov_scale - view_forward);

    // Intersect with y=0 plane
    let t = (0.0 - camera.position.y) / ray_dir.y;
    let world_pos = camera.position + ray_dir * t;

    // Calculate derivatives BEFORE any branching
    let coord = world_pos.xz;
    let derivative = fwidth(coord);

    // NOW check for invalid intersections
    let is_parallel = abs(ray_dir.y) < 0.001;
    let is_behind = t < 0.0;

    if (is_parallel || is_behind) {
        // Discard pixels that don't hit the ground
        var output: FragmentOutput;
        output.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        output.depth = 1.0; // Far plane
        return output;
    }

    // Simple approach: just cut off when derivative gets too large to avoid moire
    let deriv_len = length(derivative);

    // Minor grid (every 1 unit) - cut off when too far
    let grid = abs(fract(coord - 0.5) - 0.5) / derivative;
    let line_minor = min(grid.x, grid.y);
    let minor_alpha = (1.0 - min(line_minor, 1.0)) * GRID_ALPHA_MINOR;

    // Major grid (every 10 units) - cut off when too far
    let grid_major = abs(fract(coord / 10.0 - 0.5) - 0.5) / (derivative / 10.0);
    let line_major = min(grid_major.x, grid_major.y);
    let major_alpha = (1.0 - min(line_major, 1.0)) * GRID_ALPHA_MAJOR;

    // Axes
    let x_axis_dist = abs(world_pos.z) / derivative.y;
    let z_axis_dist = abs(world_pos.x) / derivative.x;
    let x_axis_alpha = (1.0 - min(x_axis_dist, 1.0)) * GRID_ALPHA_AXIS;
    let z_axis_alpha = (1.0 - min(z_axis_dist, 1.0)) * GRID_ALPHA_AXIS;

    // Priority-based color selection with background
    var final_color = GRID_COLOR_BACKGROUND;
    var final_alpha = GRID_ALPHA_BACKGROUND;

    // Layer lines on top of background (higher priority = drawn on top)
    if (minor_alpha > 0.01) {
        final_color = GRID_COLOR_MINOR;
        final_alpha = GRID_ALPHA_MINOR;
    }

    if (major_alpha > 0.01) {
        final_color = GRID_COLOR_MAJOR;
        final_alpha = GRID_ALPHA_MAJOR;
    }

    if (z_axis_alpha > 0.01) {
        final_color = GRID_COLOR_Z_AXIS;
        final_alpha = GRID_ALPHA_AXIS;
    }

    if (x_axis_alpha > 0.01) {
        final_color = GRID_COLOR_X_AXIS;
        final_alpha = GRID_ALPHA_AXIS;
    }

    // Calculate depth manually since view_proj matrix might have same issues as inv_view_proj
    // Transform world position to view space
    let view_pos = camera.view * vec4<f32>(world_pos.x, world_pos.y, world_pos.z, 1.0);
    let view_z = -view_pos.z;  // View space Z (positive = in front of camera)

    // Project to NDC depth using projection matrix properties
    let clip_pos = camera.proj * view_pos;
    let ndc_depth = clip_pos.z / clip_pos.w;

    // Apply bias to prevent z-fighting in both directions
    // Above grid: positive bias (push grid farther) so objects appear in front
    // Below grid: negative bias (pull grid closer) so grid appears in front
    let camera_above_grid = camera.position.y > 0.0;
    let depth_bias = select(-0.0001, 0.0001, camera_above_grid);
    let depth = ndc_depth + depth_bias;

    var output: FragmentOutput;
    output.color = vec4<f32>(final_color, final_alpha);
    output.depth = depth;
    return output;
}
