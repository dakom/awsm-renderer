# Opaque material pass

- fix KHR_texture_transform causing panic
  - might need to actually implement it? "range end index 2896 out of range for slice of length 2888"

- needs to check the material meta and early-exit if it's "not me"
  - which model would be a good test for that?
    - AI prompt:

      in crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/compute.wgsl I have a check `pbr_should_run()` which calls `pbr_material_has_any_uvs()` and `pbr_material_uses_uv_count()`.

      Right now these are placeholders, and they should be buggy, but I'm not sure which gltf test model would be the easiest to see problems here.

      The overall idea is that I have a single texture output from the visibility pass that the opaque material pass (compute shader mentioned above) is reading from, where each fragment contains different material info from the visibility pass.

      So, different passes of the compute shader should only work on the fragments that are relevant for that compute pass... if every pass writes every output pixel, it will overwrite other cells.

      I have cloned all the official gltf test models locally in `media/glTF-Sample-Assets/Models`. Please do not waste time inspecting all the binaries - but the `.gltf` files are generally there and informative, as are READMEs.

      Note that I want the *simplest reproducible case* that exposes the bug, not necessarily the most complex model that uses all the features.

      So, for example, a model that uses an `extension` is not as ideal as a model without extensions, even if the extension is not relevant to the bug itself.
      Similarly, models that use normals and lighting calculations are not as ideal as models that just use base color textures, even if the bug is not related to lighting.

    - response:

        A good minimal repro is still in the Khronos pack: load media/glTF-Sample-Assets/Models/TextureTransformMultiTest/glTF/TextureTransformMultiTest.gltf, but focus on the EmissionSample
        column only. That mesh (EmissionSample primitive, media/.../TextureTransformMultiTest.gltf:1300) carries normals, tangents, and a single UV set. Its material (EmissionSampleMat, media/.../
        TextureTransformMultiTest.gltf:627) has no baseColorTexture, only an emissive map. Because the placeholder pbr_material_has_any_uvs/pbr_material_uses_uv_count only look at has_base_color_texture, the
        two-UV/tangent-aware shader variant bails out, and the UV‑less variant runs instead. That variant assumes a layout without tangents, so _texture_uv_per_vertex reads the tangent xyz as if they were
        UVs – you’ll see the “Sample” checkmark turn into streaks/garbage even when the camera keeps the other geometry out of the way.

        The file does list KHR_texture_transform (other rows use it), but the specific EmissionSampleMat we’re using doesn’t touch any extensions, so you don’t need transform support to repro. If you want a
        completely extension-free copy, duplicate the GLTF and keep only that mesh/material pair; the artefact still appears until pbr_material_has_any_uvs and pbr_material_uses_uv_count get updated to flag
        emissive-only materials as UV users.

        • With KHR_texture_transform disabled the rows that relied on that extension will (predictably) fall back to their untransformed look, but the regression you’re chasing is still visible in the emission
          row and you can treat it as a pass/fail indicator:

          Failing (current placeholder logic).

          - In EmissionSample (media/glTF-Sample-Assets/Models/TextureTransformMultiTest/glTF/TextureTransformMultiTest.gltf:1300), the short “Sample” column should show a clean green checkmark. Instead you’ll
            see streaked/garbled colors or repeated blocks, because the UV-less shader variant is writing over those pixels and sampling tangents as if they were UVs.
          - The UV0/UV1 emissive columns both look wrong or identical – the “UV1” box never shows the rotated/offset pattern its texCoord: 1 map encodes.

          Passing (after pbr_material_has_any_uvs/pbr_material_uses_uv_count are fixed).

          - The Sample column regains the crisp green checkmark, matching the reference screenshot in TextureTransformMultiTest/README.md.
          - The UV0 and UV1 emissive boxes differ: UV0 shows the baseline checkmark, UV1 shows the variant that uses TEXCOORD_1. Even with transforms disabled the two columns still render distinct orientations
            because the underlying UV sets differ.

          So: disable the extension, fly the camera back to face the wall, and look at those three emission boxes. If the Sample patch is smeared or the UV1 patch doesn’t differ from UV0, the bug is still
          present. Once both emissive boxes look like deliberate checkmarks and the Sample patch is clean, you’ve got a passing run.


  - material_has_any_uvs / material_uses_uv_count
- get Texture settings test working
  - wrap modes
  - filter modes
  - http://127.0.0.1:9080/app/model/TextureSettings
- alpha cutoff (not full alpha blend test, just cutoff, rest depends on transparent pass)
- get basic lighting working
  - Calculate world position
  - don't worry about morphed normals yet


# Transparent material pass
- complete getting alpha blend mode working again

# Load mip level in compute shader pass

# Ensure normals are being recalculated
  - test with some model that has morphs
  - test with some model that has skins


# Get started with light culling pass

1. Write light
2. Use in opaque material pass

# get rid of 256 byte alignment for mesh meta?

- maybe only necessary for uniforms?

# Multithreading

Dynamic/Uniform storages could be SharedArrayBuffer
Requires more design/thought (don't want to expose raw manipulation)


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
