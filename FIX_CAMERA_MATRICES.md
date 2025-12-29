# ✅ FIXED: Camera Matrix Issue

**Status:** RESOLVED (with known limitation)
**Root Cause:** WGSL uniform buffer struct alignment - mixed vec3/u32 types caused implicit padding
**Solution:** Changed all struct members to 16-byte aligned types (vec4, mat4, arrays)

**Current Status:**
- ✅ Matrix multiplication works correctly
- ✅ Frustum rays populated correctly (no longer all zeros)
- ✅ Perspective camera grid rendering works perfectly
- ⚠️ Orthographic camera grid has distortion at angles (TODO: investigate derivative calculation or use different approach)

## Solution Summary

The camera uniform buffer struct had **implicit padding** between members due to WGSL alignment rules. Using mixed types (`vec3<f32>` + `u32`) caused the WGSL compiler to insert hidden padding bytes, misaligning all subsequent fields.

**The Fix:**
1. Changed `position` from `vec3<f32>` to `vec4<f32>` (using `.xyz` in shaders)
2. Changed `frame_count` from standalone `u32` to `vec4<u32>` (using `.x` for the value)
3. Added explicit padding at the end to reach 512 bytes (required by compute pipelines)
4. **Result:** All struct members are now 16-byte aligned with NO implicit padding gaps

**Final Struct Layout (512 bytes, all 16-byte aligned):**
```wgsl
struct CameraUniform {
    view: mat4x4<f32>,                  // 64 bytes
    proj: mat4x4<f32>,                  // 64 bytes
    view_proj: mat4x4<f32>,             // 64 bytes
    inv_view_proj: mat4x4<f32>,         // 64 bytes
    inv_proj: mat4x4<f32>,              // 64 bytes
    inv_view: mat4x4<f32>,              // 64 bytes
    position: vec4<f32>,                // 16 bytes (.xyz = position, .w unused)
    frame_count_and_padding: vec4<u32>, // 16 bytes (.x = frame_count, .yzw unused)
    frustum_rays: array<vec4<f32>, 4>,  // 64 bytes
    _padding_end: array<vec4<f32>, 2>,  // 32 bytes
}                                       // Total: 512 bytes
```

**Files Modified:**
- `crates/renderer/src/camera.rs` - Buffer layout (464 → 512 bytes)
- `crates/frontend/src/pages/app/scene/editor/shaders/grid.wgsl` - Struct definition + unprojection code
- `crates/renderer/src/render_passes/shared/shader/geometry_and_all_material_wgsl/camera.wgsl` - Struct definition
- `crates/renderer/src/render_passes/material_transparent/shader/material_transparent_wgsl/fragment.wgsl` - Use `.xyz`
- `crates/renderer/src/render_passes/material_opaque/shader/material_opaque_wgsl/helpers/standard.wgsl` - Use `.xyz`

## Original Problem Summary

During implementation of the ground grid shader (`crates/frontend/src/pages/app/scene/editor/shaders/grid.wgsl`), we discovered that standard matrix-vector multiplication using the camera's inverse view-projection matrix (`inv_view_proj`) does not work correctly. The multiplication produces constant results across all pixels instead of varying values based on screen position (NDC coordinates).

## ~~Current Workaround~~ (REMOVED - No longer needed!)

**Previous workaround** (now replaced with proper matrix unprojection):
- Was manually constructing ray directions from view matrix basis vectors
- Worked but was not standard and lacked proper FOV/aspect ratio handling
- **Now using proper unprojection:**

```wgsl
// Unproject NDC to world space using inverse view-projection matrix
let clip_near = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
var world_near = camera.inv_view_proj * clip_near;
world_near = world_near / world_near.w;

let clip_far = vec4<f32>(ndc.x, ndc.y, 1.0, 1.0);
var world_far = camera.inv_view_proj * clip_far;
world_far = world_far / world_far.w;

let ray_origin = world_near.xyz;
let ray_dir = normalize(world_far.xyz - world_near.xyz);
```

**Location:** `crates/frontend/src/pages/app/scene/editor/shaders/grid.wgsl` lines ~75-87

## What We Discovered During Debug Session

### Symptoms

1. **Matrix multiplication produces constant output:**
   - `camera.inv_view_proj * clip_pos` returned the same value for all pixels
   - Even though `ndc` (normalized device coordinates) varied correctly across the screen
   - Individual matrix column values were readable and updating with camera movement

2. **Manual component multiplication also failed:**
   ```wgsl
   let result = camera.inv_proj[0].x * ndc.x;
   ```
   This DID work (produced left-to-right gradient), but full matrix multiplication didn't.

3. **The `w` component issue:**
   - After matrix multiplication, the `.w` component was constant across all pixels
   - This suggests the matrix rows/columns that should multiply with NDC values were not being accessed correctly

### What Works

- Reading individual matrix components: ✅
- Camera uniform updates (matrices change with camera movement): ✅
- NDC varying across pixels: ✅
- Scalar multiplication with matrix elements: ✅
- Built-in WGSL matrix * vector multiplication: ❌ (produces constant output)

### Tested Approaches That Failed

1. Using `camera.inv_view_proj * clip_pos` directly
2. Transposing the matrix first: `transpose(camera.inv_view_proj) * clip_pos`
3. Manual matrix-vector multiplication writing out all components
4. Splitting into two steps: `inv_proj` then `inv_view`

## Camera Buffer Layout

**Location:** `crates/renderer/src/camera.rs` 

**Note:** The frustum rays were reading as all zeros during our debug session, suggesting a potential buffer layout/alignment issue.

## Shader Uniform Declaration

**Location:** `crates/frontend/src/pages/app/scene/editor/shaders/grid.wgsl` lines 1-13

```wgsl
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
```

## What Needs Investigation

1. **Buffer layout alignment:**
   - Are the matrices being written with correct byte alignment?
   - WGSL matrices have specific alignment requirements (16-byte aligned columns)
   - Check if Rust's `to_cols_array()` matches WGSL's memory layout expectations

2. **Matrix storage order:**
   - WGSL uses column-major order
   - Rust's glam uses column-major too, but verify the upload is correct
   - The fact that individual component access works but multiplication doesn't suggests the data is there but not being accessed correctly during multiplication

3. **Frustum rays being zero:**
   - This suggests the buffer layout might be misaligned after the matrices
   - If padding is wrong, subsequent data gets corrupted

4. **Why scalar multiplication works but matrix multiplication doesn't:**
   - When we did `camera.inv_proj[0].x * ndc.x` it worked
   - But `camera.inv_proj * clip_pos` didn't
   - This is very suspicious and suggests a compiler or matrix access issue

## Files to Investigate

1. **Camera buffer upload code**:
   - Where matrices are converted to bytes and uploaded
   - The `write_f32_slice()` function
   - Check alignment between writes

2. **Shader uniform declaration:**
   - Verify alignment matches Rust side

3. **Bind group creation:**
   - `crates/frontend/src/pages/app/scene/editor/pipelines` around line 73 (where grid bind group is created)

## Success Criteria

The fix is complete when:

1. Standard unprojection works:
   ```wgsl
   let clip_near = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
   var world_near = camera.inv_view_proj * clip_near;
   world_near = world_near / world_near.w;
   ```
   This should produce **varying** world positions across the screen.

2. Can remove the workaround basis vector extraction code

3. Frustum rays are no longer all zeros (bonus - would enable cleaner raycasting)

## Additional Context

- WebGPU/WGSL environment
- Using right-handed coordinate system
- Perspective projection with 45° FOV
- This is blocking proper camera-based raycasting in other parts of the editor

## Analysis of Buffer Layout (Updated)

### WGSL Uniform Buffer Alignment Rules

In WGSL uniform address space:
- `mat4x4<f32>`: size = 64 bytes, alignment = 16 bytes (treated as array of 4 vec4s)
- `vec3<f32>`: size = 12 bytes, **alignment = 16 bytes** (same as vec4 in uniform space!)
- `u32`: size = 4 bytes, alignment = 4 bytes
- `array<vec4<f32>, 4>`: alignment = 16 bytes (alignment of element type)

### Current Buffer Layout Calculation

```
Offset  | Member            | Size  | Alignment | End Offset
--------|-------------------|-------|-----------|------------
0       | view              | 64    | 16        | 64
64      | proj              | 64    | 16        | 128
128     | view_proj         | 64    | 16        | 192
192     | inv_view_proj     | 64    | 16        | 256
256     | inv_proj          | 64    | 16        | 320
320     | inv_view          | 64    | 16        | 384
384     | position (vec3)   | 12    | 16        | 396
396     | frame_count (u32) | 4     | 4         | 400
400     | frustum_rays      | 64    | 16        | 464
```

**Layout appears correct** - all alignments are satisfied.

### Potential Root Causes

1. **WGSL Struct Packing Issue**: WGSL may be auto-inserting padding differently than expected
   - The `vec3<f32>` followed by `u32` might cause unexpected padding
   - Solution: Try using `vec4<f32>` for position (with w component unused) to ensure clean 16-byte alignment

2. **Matrix Storage Order Mismatch**: While glam uses column-major and WGSL expects column-major, there might be a subtle difference in how the bytes are interpreted
   - Need to verify the matrix elements are in the correct order

3. **Compiler Bug**: The WGSL compiler might have an issue with matrix-vector multiplication when the struct has mixed-size members
   - The fact that individual component access works but multiplication doesn't is highly suspicious

### Recommended Debug Approach

1. **Test with explicit padding**: Change WGSL struct to use `vec4<f32>` for position instead of `vec3<f32>`
2. **Add debug visualization**: Create a test that outputs individual matrix components vs multiplication result
3. **Verify buffer contents**: Add Rust-side logging to dump the exact bytes being uploaded
4. **Test simplified struct**: Create a minimal test case with just one matrix to isolate the issue

## Quick Test to Verify Fix

Add this to the fragment shader and check if you see a gradient:

```wgsl
let clip_pos = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
var world_pos = camera.inv_view_proj * clip_pos;
world_pos = world_pos / world_pos.w;

// Should show a gradient if working
return vec4<f32>(
    world_pos.x * 0.1 + 0.5,
    world_pos.y * 0.1 + 0.5,
    0.5,
    1.0
);
```

If you see a solid color that changes with camera movement but doesn't vary across the screen - the bug still exists.
If you see a gradient that varies across the screen - it's fixed!
