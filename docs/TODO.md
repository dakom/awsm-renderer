# Set MeshInfo as uniform buffer

- it's all prepared, just bind!
- get morphs working again
    - use uniform data for presence and loop length
- get skins working again
    - use unform data for presence and loop length

- unroll the common cases, fall back to for-loop only if those aren't met

# Get started with opaque material pass 

2. Pass material offset from geometry pass
3. Calculate world position
4. Write basic material

# Load mip level in compute shader pass

# Change transform and material binding to storage in geometry pass (prepare for unified draw call)

# Get started with light culling pass

1. Write light
2. Use in opaque material pass

# Unified draw call (see below)


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

# Draw Calls

### Data structures:

```rust
struct DrawMetadata {
    material_offset: u32, // offset into material buffer
    base_index: u32,
    index_count: u32,
    base_vertex: u32,

    vertex_stride: u32,
    attribute_mask: u32, // Bitmask: // 0b01 = normal, 0b10 = tangent, etc.
}

struct IndirectDraw {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}
```

### Shader code (pseudocode example):

```wgsl 
@group(0) @binding(0)
var<storage, read> meta_buffer: array<DrawMetadata>;

// This contains all the attribute data per-vertex
// It allows for optional attributes, but always follows the same order
// for example: position (vec3) → normal? (vec3) → tangent? (vec4) → ...
@group(0) @binding(1)
var<storage, read> vertex_buffer: array<f32>;

// This is a buffer for transform matrix per-instance
@group(0) @binding(2)
var<storage, read> u_transform: array<mat4x4<f32>>;

// skin transformation matrices
@group(0) @binding(3)
var<storage, read> skin_joint_mat: array<mat4x4f>; 

@group(0) @binding(4)
var<storage, read> morph_weights: array<f32>;

// follows the same interleaving approach as `vertex_buffer`
@group(0) @binding(5)
var<storage, read> morph_values: array<f32>; 

@builtin(vertex_index)
var<in> vertex_index: u32;

@builtin(instance_index)
var<in> instance_index: u32;

struct DrawMetadata {
    // the starting index into vertex_buffer
    base_vertex: u32,

    // the stride in floats of all attributes for this vertex
    // that will be 3 for each position (vec3), plus 4 for each tangent (vec4), etc.
    vertex_stride: u32, 

    // tells us which attributes are present, as a bitmask
    // 0b01 = normal, 0b10 = tangent, 0b100 = skin, etc. 
    attribute_mask: u32, 

    // for writing from the visibility pass, used in later passes
    material_offset: u32, 

    // the index into the transform buffer
    transform_index: u32,

    // the index into the skin_joint_mat buffer
    skin_index: u32,
    // the number of joints in this skin
    skin_joint_count: u32,

    // the number of morph targets
    morph_targets: u32,
    // the index into the morph_weights buffer
    morph_weight_index: u32,
    // the index into the morph_values buffer
    morph_value_index: u32,
    // the stride in floats of all morphs for this vertex
    morph_stride: u32,
    // 0b01 = position, 0b10 = normal, 0b100 = tangent, etc. 
    morph_mask: u32,
}

const ATTR_NORMAL   = 0b01u;
const ATTR_TANGENT  = 0b10u;
const ATTR_SKIN     = 0b100u;

const MORPH_POSITION    = 0b01u;
const MORPH_NORMAL      = 0b10u;
const MORPH_TANGENT     = 0b100u;

fn vs_main(...) -> ... {
    // Load the metadata for this mesh/instance
    let meta = meta_buffer[instance_index];

    let transform = u_transform[meta.transform_index];

    // Get the start of this vertex's data
    var curr_index = meta.base_vertex + (vertex_index * meta.vertex_stride);

    var position = read_and_advance_vertex_vec3(&curr_index);

    // vertex data is predictably interleaved, we can rely on these always being the same order
    if ((meta.attribute_mask & ATTR_NORMAL) != 0u) {
        let normal = read_and_advance_vertex_vec3(&curr_index);
    }

    if ((meta.attribute_mask & ATTR_TANGENT) != 0u) {
        let tangent = read_and_advance_vertex_vec4(&curr_index);
    }

    if ((meta.attribute_mask & ATTR_SKIN) != 0u) {
        var skinned_pos = vec4<f32>(0.0, 0.0, 0.0, 0.0);

        if (meta.skin_joint_count == 4u) {
            // Unrolled fast path for typical case
            let joint_index_0 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_0 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_1 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_1 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_2 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_2 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_3 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_3 = read_and_advance_vertex_f32(&curr_index);

            let mat_0 = skin_joint_mat[meta.skin_index + joint_index_0];
            let mat_1 = skin_joint_mat[meta.skin_index + joint_index_1];
            let mat_2 = skin_joint_mat[meta.skin_index + joint_index_2];
            let mat_3 = skin_joint_mat[meta.skin_index + joint_index_3];

            skinned_pos = (mat_0 * vec4<f32>(position, 1.0)) * joint_weight_0 +
                (mat_1 * vec4<f32>(position, 1.0)) * joint_weight_1 +
                (mat_2 * vec4<f32>(position, 1.0)) * joint_weight_2 +
                (mat_3 * vec4<f32>(position, 1.0)) * joint_weight_3;
        } else if (meta.skin_joint_count == 8u) {
            // Also pretty typical 
            let joint_index_0 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_0 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_1 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_1 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_2 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_2 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_3 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_3 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_4 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_4 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_5 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_5 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_6 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_6 = read_and_advance_vertex_f32(&curr_index);
            let joint_index_7 = read_and_advance_vertex_u32(&curr_index);
            let joint_weight_7 = read_and_advance_vertex_f32(&curr_index);

            let mat_0 = skin_joint_mat[meta.skin_index + joint_index_0];
            let mat_1 = skin_joint_mat[meta.skin_index + joint_index_1];
            let mat_2 = skin_joint_mat[meta.skin_index + joint_index_2];
            let mat_3 = skin_joint_mat[meta.skin_index + joint_index_3];
            let mat_4 = skin_joint_mat[meta.skin_index + joint_index_4];
            let mat_5 = skin_joint_mat[meta.skin_index + joint_index_5];
            let mat_6 = skin_joint_mat[meta.skin_index + joint_index_6];
            let mat_7 = skin_joint_mat[meta.skin_index + joint_index_7];

            skinned_pos = (mat_0 * vec4<f32>(position, 1.0)) * joint_weight_0 +
                (mat_1 * vec4<f32>(position, 1.0)) * joint_weight_1 +
                (mat_2 * vec4<f32>(position, 1.0)) * joint_weight_2 +
                (mat_3 * vec4<f32>(position, 1.0)) * joint_weight_3 +
                (mat_4 * vec4<f32>(position, 1.0)) * joint_weight_4 +
                (mat_5 * vec4<f32>(position, 1.0)) * joint_weight_5 +
                (mat_6 * vec4<f32>(position, 1.0)) * joint_weight_6 +
                (mat_7 * vec4<f32>(position, 1.0)) * joint_weight_7;

        } else {
            // Generic loop fallback
            for (var count = 0u; count < meta.skin_joint_count; count = count + 1u) {
                // joint_index and joint_weight are direct vertex attributes
                let joint_index = read_and_advance_vertex_u32(&curr_index);
                let joint_weight = read_and_advance_vertex_f32(&curr_index);

                // but joint_mat is in a different buffer
                let joint_mat = skin_joint_mat[meta.skin_index + joint_index];

                // accumulate
                skinned_pos += (joint_mat * vec4<f32>(position, 1.0)) * joint_weight;
            }
        }


        position = skinned_pos.xyz;
    }

    if (meta.morph_mask != 0u) {
        for (var morph_target = 0u; morph_target < meta.morph_targets; morph_target = morph_target + 1u) {
            var morph_weight = morph_weights[meta.morph_weight_index + morph_target];

            var value_index = meta.morph_value_index + (morph_target * meta.morph_stride);

            if ((meta.morph_mask & MORPH_POSITION) != 0u) {
                let morph_position = read_and_advance_morph_vec3(&value_index);

                position += morph_weight * morph_position; 
            }

            // same for other morphs
        }
    }

    ...
}

fn read_and_advance_vertex_f32(curr_index: ptr<function, u32>) -> f32 {
    let result = vertex_buffer[*curr_index];
    *curr_index += 1u;
    return result;
}

fn read_and_advance_vertex_u32(curr_index: ptr<function, u32>) -> u32 {
    let result = u32(vertex_buffer[*curr_index]);
    *curr_index += 1u;
    return result;
}

fn read_and_advance_vertex_vec3(curr_index: ptr<function, u32>) -> vec3<f32> {
    let result = vec3<f32>(vertex_buffer[*curr_index], vertex_buffer[*curr_index + 1], vertex_buffer[*curr_index + 2]);
    *curr_index += 3u;
    return result;
}

fn read_and_advance_vertex_vec4(curr_index: ptr<function, u32>) -> vec4<f32> {
    let result = vec4<f32>(vertex_buffer[*curr_index], vertex_buffer[*curr_index + 1], vertex_buffer[*curr_index + 2], vertex_buffer[*curr_index + 3]);
    *curr_index += 4u;
    return result;
}

fn read_and_advance_morph_vec3(curr_index: ptr<function, u32>) -> vec3<f32> {
    let result = vec3<f32>(morph_values[*curr_index], morph_values[*curr_index + 1], morph_values[*curr_index + 2]);
    *curr_index += 3u;
    return result;
}


```

### Process

One-time, when loading mesh data:

1. Store all per-vertex data in storage buffers (that's positions, normals, tangents, etc.)
2. Store all per-mesh data in storage buffers (transforms, skin/morph matrices, etc.)

Each frame

1. Collect a list of visible meshes and sort them front-to-back
1. Create 2 storage buffers: `meta_buffer` and `instance_buffer`
2. For each visible mesh, push its data into those buffers
3. Run a single draw-call using `draw_indexed_indirect` that takes the `instance_buffer`

The shader code will then work as above (loads the `meta_buffer` from `instance_index`, gets called with the right number of vertices) 

### Transparency

Transparent passes are similar, just that the shader will differ much more per-material, so the above process needs to be done once per material (e.g. pbr, unlit, etc.), specifically - grouped by RenderPipeline.