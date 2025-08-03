# Finish texture atlas writer
 - upload the uniform with layer_index etc.
 - generate mipmap (have a flag on texture_atlas, use same code as ImageData::create_texture)

# Get started with opaque material pass 

1. Sanity check - only draw where there's geometry

... then
2. Read material at material_offset
3. Calculate world position
4. Write basic material

# Get started with light culling pass

1. Write light
2. Use in opaque material pass


# World Position Reconstruction Math: 

A common way to implement this is to reconstruct the view-space position first, then transform it to world space. In your shading shader:

Get the pixel's UV coordinate (e.g., from builtin(global_invocation_id).xy / screen_dimensions).

Calculate the view-space ray direction for that pixel (see below)

The view-space position is then simply: vec3f view_pos = view_ray_direction * linear_eye_space_depth;

The world-space position is: vec3f world_pos = (inverse_view_matrix * vec4f(view_pos, 1.0)).xyz;

# View-space ray direction math:

### CPU side
You only need to do this once whenever the projection matrix changes (e.g., FOV change, window resize).

The idea is to define the four corners of the screen in Normalized Device Coordinates (NDC) and "un-project" them back into view space using the inverse of your projection matrix.

use glam::{Mat4, Vec3, Vec4};

// Your camera's projection matrix
let projection_matrix: Mat4 = /* ... your projection matrix ... */;
let inverse_projection: Mat4 = projection_matrix.inverse();

// Define corners in NDC. Z=1 is the far plane in a standard [-1, 1] clip space.
// We use Vec4 because matrix multiplication needs it.
let ndc_corners = [
    Vec4::new(-1.0, -1.0, 1.0, 1.0), // Bottom-left
    Vec4::new( 1.0, -1.0, 1.0, 1.0), // Bottom-right
    Vec4::new(-1.0,  1.0, 1.0, 1.0), // Top-left
    Vec4::new( 1.0,  1.0, 1.0, 1.0), // Top-right
];

let mut frustum_corners_view_space: [Vec3; 4] = [Vec3::ZERO; 4];

for i in 0..4 {
    // Transform from clip space to view space
    let corner_view_unhomo = inverse_projection * ndc_corners[i];
    // Perform perspective divide to get the 3D coordinate in view space
    frustum_corners_view_space[i] = corner_view_unhomo.truncate() / corner_view_unhomo.w;
}

// Now `frustum_corners_view_space` contains the four Vec3 rays.
// Send this array of 4 vectors to your compute shader as a uniform.

### GPU side
// Uniforms received from the CPU
@group(0) @binding(0) var<uniform> camera: CameraUniforms;
// struct CameraUniforms {
//   ...
//   frustum_corners: array<vec3<f32>, 4>,
//   ...
// };

@group(1) @binding(1) var depth_texture: texture_2d<f32>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let screen_dims = vec2<f32>(textureDimensions(depth_texture));
    
    // 1. Calculate the pixel's UV coordinate (0.0 to 1.0)
    let uv = (vec2<f32>(id.xy) + vec2(0.5, 0.5)) / screen_dims;

    // 2. Bilinearly interpolate between the 4 frustum corner rays using the UV
    //    This gives us the specific ray for this pixel.
    let bottom_ray = mix(camera.frustum_corners[0], camera.frustum_corners[1], uv.x);
    let top_ray    = mix(camera.frustum_corners[2], camera.frustum_corners[3], uv.x);
    let pixel_ray  = mix(bottom_ray, top_ray, uv.y);

    // 3. Load the linear eye-space depth stored from the geometry pass
    let linear_depth = textureLoad(depth_texture, id.xy, 0).r;

    // 4. Calculate the final 3D view-space position
    //    This is the key step. We scale the ray so that its Z component
    //    matches the linear depth we stored.
    let view_position = pixel_ray * (linear_depth / pixel_ray.z);

    // Now you have the accurate view-space position of the pixel!
    // let world_position = inverse_view_matrix * vec4(view_position, 1.0);
    // ... do lighting ...
}

🤔 Why does pixel_ray * (linear_depth / pixel_ray.z) work?
pixel_ray is a vector that points from the eye to the far plane. Its length and components are relative to the far plane distance. For example, pixel_ray.z is equal to your far_plane distance.

We have linear_depth, which is the actual Z distance from the camera to the surface.

The ratio linear_depth / pixel_ray.z gives us the exact scaling factor we need. It tells us "how far along the ray" our surface point is.

Multiplying the ray by this factor scales the entire (X, Y, Z) vector proportionally, giving you the correct (x_view, y_view, z_view) position.