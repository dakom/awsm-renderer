# Mipmap Debugging Guide

**Date:** 2025-11-04
**Status:** MUCH improved - depth-based per-pixel LOD working well
**Remaining Bug:** Multiple lines reappear at far zoom distances

---

## Current Status

### ✅ What's Working
- Per-pixel LOD calculation using depth buffer reconstruction
- World position reconstruction from depth
- UV derivatives computed correctly via barycentric interpolation
- Smooth LOD transitions across surfaces (no triangle seams)
- Repeating textures work correctly
- Close-up and mid-range zoom levels look good
- Standard LOD formula: `LOD = log2(max(||dUV/dx||, ||dUV/dy||) * texture_size)`

### ❌ Remaining Bugs

**Bug 1: At far zoom out distances, multiple lines come back**

This suggests the LOD is too LOW (using too detailed mip levels) when far away, causing:
- Undersampling/aliasing artifacts
- Multiple texels being visible when they should blur together
- Thin lines that should disappear becoming visible again

**Bug 2: Some textures in other test scenes are partially all white when they shouldn't be**

This suggests:
- LOD calculation producing invalid values (NaN, infinity, or extreme values)
- UV derivatives becoming degenerate (0/0, infinity)
- Depth reconstruction failing for certain geometry types
- Atlas sampling going completely out of bounds
- Possible causes: back-facing triangles, extreme grazing angles, degenerate geometry

---

## Implementation Overview

### Core Files

**1. `/crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/helpers/mipmap.wgsl`**
- Lines 56-77: `reconstruct_world_position()` - Depth buffer to world space
- Lines 79-172: `compute_uv_derivatives_from_depth()` - Per-pixel UV derivatives
- Lines 267-310: `compute_texture_lod_from_depth()` - LOD calculation
- Lines 174-245: `pbr_get_mipmap_levels()` - Public API

**2. `/crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/compute.wgsl`**
- Lines 316-327: Call site for `pbr_get_mipmap_levels()`

### Key Constants
```wgsl
const MIPMAP_GLOBAL_LOD_BIAS : f32 = -0.5;  // Line 18 of mipmap.wgsl
const MIPMAP_CLAMP_EPSILON   : f32 = 1e-4;
const MIPMAP_ATLAS_PADDING   : f32 = 8.0;
```

### Algorithm Flow
```
1. Read depth values (center + right + down neighbors)
   depth_center = textureLoad(depth_tex, coords, 0)
   depth_x = textureLoad(depth_tex, coords + vec2(1,0), 0)
   depth_y = textureLoad(depth_tex, coords + vec2(0,1), 0)

2. Reconstruct world positions for each depth sample
   world_center = reconstruct_world_position(pixel_center, depth_center, inv_view_proj, screen_size)
   world_x = reconstruct_world_position(pixel_center + vec2(1,0), depth_x, ...)
   world_y = reconstruct_world_position(pixel_center + vec2(0,1), depth_y, ...)

3. Compute world-space derivatives
   dWorld_dx = world_x - world_center
   dWorld_dy = world_y - world_center

4. Get triangle vertices in world space
   v0_world = (world_model * vec4(os_vertices.p0, 1.0)).xyz
   v1_world = (world_model * vec4(os_vertices.p1, 1.0)).xyz
   v2_world = (world_model * vec4(os_vertices.p2, 1.0)).xyz

5. Solve barycentric system to get UV derivatives
   - Build edge vectors: e01_world, e02_world, e01_uv, e02_uv
   - Project onto triangle plane
   - Solve 2x2 system for barycentric derivatives
   - Chain rule: dUV/dScreen = dUV/dWorld * dWorld/dScreen

6. Compute LOD
   dudx_texels = dudx * texture_width
   dvdx_texels = dvdx * texture_height
   (same for dy)

   rho_x = sqrt(dudx_texels² + dvdx_texels²)
   rho_y = sqrt(dudy_texels² + dvdy_texels²)
   rho = max(rho_x, rho_y)

   lod = log2(max(rho, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS
   lod = clamp(lod, 0.0, max_lod)
```

---

## Debugging Steps

### Step 1: Add LOD Visualization

**Location:** `mipmap.wgsl:compute_texture_lod_from_depth()` around line 300

Add debug output to visualize LOD values:
```wgsl
// Before return, store for debug
// debug_lod = lod;  // Store in global if needed

// Or add to compute.wgsl after pbr_get_mipmap_levels call:
let debug_lod = texture_lods.base_color;
// Visualize: red = 0, yellow = 2, green = 4, cyan = 6, blue = 8+
let normalized_lod = saturate(debug_lod / 8.0);
color = vec4<f32>(
    mix(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), normalized_lod),
    1.0
);
```

**Expected behavior:**
- Close up: Red (LOD 0-1)
- Mid range: Yellow/Green (LOD 2-4)
- Far away: Cyan/Blue (LOD 5-8+)

**Current bug:** If lines reappear at far distances, LOD might be stuck at mid-range values (yellow/green) when it should be blue.

### Step 2: Add Gradient Magnitude Visualization

**Location:** `mipmap.wgsl:compute_texture_lod_from_depth()` after line 291

```wgsl
let rho_x = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);
let rho_y = sqrt(dudy_texels * dudy_texels + dvdy_texels * dvdy_texels);
let rho = max(rho_x, rho_y);

// DEBUG: Visualize rho
// Expect: Small when close (many texels per pixel), large when far (few texels per pixel)
// If rho is too large at far distances, LOD will be too high (too detailed)
// debug_rho = rho;
```

Visualize in compute.wgsl:
```wgsl
// After compute_texture_lod_from_depth, add debug parameter to return rho
// let normalized_rho = saturate(log2(rho) / 8.0);
// color = vec4<f32>(vec3<f32>(normalized_rho), 1.0);
```

**Expected:** Should increase smoothly as camera zooms out.

### Step 3: Check Derivative Values

Add debug output for raw derivatives:

**Location:** `mipmap.wgsl:compute_uv_derivatives_from_depth()` around line 168

```wgsl
let dudx = dw1_dx * e01_uv.x + dw2_dx * e02_uv.x;
let dudy = dw1_dy * e01_uv.x + dw2_dy * e02_uv.x;
let dvdx = dw1_dx * e01_uv.y + dw2_dx * e02_uv.y;
let dvdy = dw1_dy * e01_uv.y + dw2_dy * e02_uv.y;

// DEBUG: Check for invalid/extreme values
// if (abs(dudx) > 100.0 || abs(dvdx) > 100.0) {
//     // Derivatives are unreasonably large
// }
// if (abs(dudx) < 1e-8 && abs(dvdx) < 1e-8) {
//     // Derivatives are too small (might clamp LOD to 0)
// }
```

### Step 4: Test LOD Bias Adjustment

**Location:** `mipmap.wgsl` line 18

Current: `const MIPMAP_GLOBAL_LOD_BIAS : f32 = -0.5;`

Try different values:
- `-1.0` - More detailed (lower LOD numbers)
- `0.0` - No bias
- `+0.5` - More blurry (higher LOD numbers)
- `+1.0` - Even more blurry

**If bug persists with +1.0 bias:** The problem is not the bias, it's the LOD calculation itself.

---

## Potential Root Causes (Bug 1: Multiple Lines)

These apply to the "multiple lines reappear at far distances" bug:

### 1. **Depth Discontinuities at Far Distances** (MOST LIKELY)

At far distances with grazing angles, neighboring depth samples might have large discontinuities:
```
Far away triangle at grazing angle:
  depth_center = 0.9999
  depth_x      = 0.9995  (different triangle!)
  depth_y      = 0.9997  (edge of triangle)
```

This causes:
- World position differences to be HUGE
- Derivatives to be incorrectly large
- LOD to be incorrectly low (too detailed)

**Solution:** Add depth discontinuity detection

**Location:** `mipmap.wgsl:compute_uv_derivatives_from_depth()` after line 96

```wgsl
let depth_center = textureLoad(depth_tex, coords, 0);
let depth_x = textureLoad(depth_tex, coords + vec2<i32>(1, 0), 0);
let depth_y = textureLoad(depth_tex, coords + vec2<i32>(0, 1), 0);

// DEPTH DISCONTINUITY CHECK
// If neighboring depths differ significantly, we're at an edge
// Fall back to conservative LOD (use max available)
const DEPTH_DISCONTINUITY_THRESHOLD = 0.001; // Tune this
if (abs(depth_x - depth_center) > DEPTH_DISCONTINUITY_THRESHOLD ||
    abs(depth_y - depth_center) > DEPTH_DISCONTINUITY_THRESHOLD) {
    // At edge - return safe derivatives (or flag to use max LOD)
    return UvDerivs(0.0, 0.0, 0.0, 0.0);
}
```

Then in `compute_texture_lod_from_depth()`, check for zero derivatives:
```wgsl
let d = compute_uv_derivatives_from_depth(...);

// If derivatives are zero (edge case), use max LOD
if (d.dudx == 0.0 && d.dudy == 0.0 && d.dvdx == 0.0 && d.dvdy == 0.0) {
    let atlas = get_atlas_info(tex.atlas_index);
    return max(atlas.levels_f - 1.0, 0.0);
}
```

### 2. **Grazing Angle Amplification**

At shallow angles, small screen-space movements = large world-space movements:
```
Camera looking at floor from far away:
  1 pixel right = 10 meters in world space
  Small UV change = huge texel coverage
```

**Solution:** Add angle-based correction

**Location:** `mipmap.wgsl:compute_uv_derivatives_from_depth()` after line 104

```wgsl
let dWorld_dx = world_x - world_center;
let dWorld_dy = world_y - world_center;

// Get surface normal (from triangle)
let tri_normal = normalize(cross(e01_world, e02_world));

// Get view direction (from camera to surface)
let view_dir = normalize(world_center - camera.position);

// Compute angle between view and surface
let cos_angle = abs(dot(view_dir, tri_normal));

// At grazing angles (cos_angle near 0), scale derivatives down
// to prevent over-detailing
const MIN_GRAZING_ANGLE = 0.1; // ~84 degrees
if (cos_angle < MIN_GRAZING_ANGLE) {
    let scale = cos_angle / MIN_GRAZING_ANGLE;
    // Apply scale to derivatives or return conservative LOD
}
```

### 3. **Triangle Degeneracy Check**

**Location:** `mipmap.wgsl:compute_uv_derivatives_from_depth()` line 149

Current check:
```wgsl
if (abs(det) < 1e-8) {
    return UvDerivs(0.0, 0.0, 0.0, 0.0);
}
```

Try stricter threshold:
```wgsl
if (abs(det) < 1e-6) {  // Was 1e-8
    return UvDerivs(0.0, 0.0, 0.0, 0.0);
}
```

### 4. **Max LOD Clamping**

**Location:** `mipmap.wgsl:compute_texture_lod_from_depth()` line 302

Check that max_lod is correct:
```wgsl
let atlas = get_atlas_info(tex.atlas_index);
let max_lod = max(atlas.levels_f - 1.0, 0.0);
lod = clamp(lod, 0.0, max_lod);

// DEBUG: Ensure atlas has enough mip levels
// For 256x256 texture: should have ~8 mip levels
// If atlas.levels_f is too small, we can't blur enough
```

Check mipmap generation to ensure all levels exist.

### 5. **Out-of-Bounds UV Clamping**

**Location:** `mipmap.wgsl:atlas_clamp_cap()` line 316

```wgsl
if (oob_u || oob_v) {
    let max_clamp_lod = log2(max(MIPMAP_ATLAS_PADDING - 1.0, 1.0));
    lod = min(lod, max_clamp_lod);
}
```

This limits LOD to `log2(7) ≈ 2.8` for out-of-bounds UVs.

**Debug:** Check if UVs are considered out-of-bounds at far distances:
```wgsl
// Add debug output
if (oob_u || oob_v) {
    // Flag this case for visualization
}
```

---

## Potential Root Causes (Bug 2: White Textures)

These apply to the "textures partially all white" bug:

### 1. **NaN/Infinity in LOD Calculation** (MOST LIKELY)

Invalid math operations can produce NaN:
```
det = 0 → divide by zero → infinity
rho = 0 → log2(0) = -infinity
sqrt(negative) → NaN
```

**Solution:** Add defensive checks

**Location:** `mipmap.wgsl:compute_texture_lod_from_depth()` around line 287

```wgsl
let dudx_texels = d.dudx * f32(tex.size.x);
let dudy_texels = d.dudy * f32(tex.size.x);
let dvdx_texels = d.dvdx * f32(tex.size.y);
let dvdy_texels = d.dvdy * f32(tex.size.y);

// CHECK FOR INVALID VALUES
if (!isFinite(dudx_texels) || !isFinite(dudy_texels) ||
    !isFinite(dvdx_texels) || !isFinite(dvdy_texels)) {
    // Return safe default LOD
    return 0.0; // Or use max LOD
}

let rho_x = sqrt(dudx_texels * dudx_texels + dvdx_texels * dvdx_texels);
let rho_y = sqrt(dudy_texels * dudy_texels + dvdy_texels * dvdy_texels);
let rho = max(rho_x, rho_y);

// CHECK RHO VALIDITY
if (!isFinite(rho) || rho <= 0.0) {
    return 0.0; // Safe default
}

var lod = log2(max(rho, 1e-6)) + MIPMAP_GLOBAL_LOD_BIAS;

// CHECK LOD VALIDITY
if (!isFinite(lod)) {
    return 0.0;
}
```

WGSL doesn't have `isFinite()`, use this instead:
```wgsl
fn is_finite(x: f32) -> bool {
    return x == x && abs(x) < 1e30; // NaN != NaN, check for huge values
}
```

### 2. **Depth Reconstruction Failure**

Certain geometry might produce invalid world positions:

**Location:** `mipmap.wgsl:reconstruct_world_position()` around line 75

```wgsl
let world_pos = inv_view_proj * clip_pos;
return world_pos.xyz / world_pos.w;

// CHECK: If w is 0 or near-zero, position is invalid
// Should add:
if (abs(world_pos.w) < 1e-6) {
    return vec3<f32>(0.0); // Flag invalid
}
```

### 3. **Back-Facing Triangles**

Computing derivatives on back-facing triangles might give nonsensical results.

**Location:** `mipmap.wgsl:compute_uv_derivatives_from_depth()` after line 141

```wgsl
let tri_normal = normalize(cross(e01_world, e02_world));

// Check if triangle is back-facing
let view_dir = normalize(world_center - camera.position);
if (dot(tri_normal, view_dir) > 0.0) {
    // Back-facing - use conservative LOD or flag error
    return UvDerivs(0.0, 0.0, 0.0, 0.0);
}
```

### 4. **Degenerate Triangles**

**Location:** `mipmap.wgsl:compute_uv_derivatives_from_depth()` line 149

Add more robust checks:
```wgsl
let det = e01_world.x * e02_world.y - e01_world.y * e02_world.x;

if (abs(det) < 1e-8) {
    return UvDerivs(0.0, 0.0, 0.0, 0.0);
}

// Also check edge lengths
let e01_len = length(e01_world);
let e02_len = length(e02_world);
if (e01_len < 1e-6 || e02_len < 1e-6) {
    // Degenerate triangle
    return UvDerivs(0.0, 0.0, 0.0, 0.0);
}

// Check UV edges too
let e01_uv_len = length(e01_uv);
let e02_uv_len = length(e02_uv);
if (e01_uv_len < 1e-6 && e02_uv_len < 1e-6) {
    // No UV variation - use mip 0
    return UvDerivs(0.0, 0.0, 0.0, 0.0);
}
```

### 5. **Zero Derivative Handling**

**Location:** `mipmap.wgsl:compute_texture_lod_from_depth()` after getting derivatives

```wgsl
let d = compute_uv_derivatives_from_depth(...);

// If all derivatives are zero, decide what to do
if (d.dudx == 0.0 && d.dudy == 0.0 && d.dvdx == 0.0 && d.dvdy == 0.0) {
    // Option 1: Use highest detail (mip 0)
    return 0.0;

    // Option 2: Use lowest detail (max mip)
    let atlas = get_atlas_info(tex.atlas_index);
    return max(atlas.levels_f - 1.0, 0.0);

    // Option 3: Use safe middle ground
    return 2.0;
}
```

Currently returns 0.0 implicitly, which might not be correct for all cases.

### 6. **Extreme LOD Values**

**Location:** `mipmap.wgsl:compute_texture_lod_from_depth()` line 302

```wgsl
lod = clamp(lod, 0.0, max_lod);

// BEFORE clamping, check for extreme values
// If lod is -100 or +100, something went wrong
if (lod < -10.0 || lod > 20.0) {
    // Use safe default
    lod = 0.0;
}

lod = clamp(lod, 0.0, max_lod);
```

---

## Debugging White Texture Bug

### Step 1: Identify Which Scenes Show White

Make note of:
- Which test scene (model name)
- Which specific textures (base color, normal, metallic, etc.)
- What percentage is white (all? half? patches?)
- Does it happen at specific camera angles?
- Does it happen at specific zoom levels?

### Step 2: Add Safety Checks

Add the defensive `is_finite()` checks described above to catch NaN/infinity.

**Create helper function in mipmap.wgsl:**

```wgsl
// Add near top of file, after structs
fn is_valid_lod(lod: f32) -> bool {
    return lod == lod && lod > -20.0 && lod < 20.0;
}

fn is_valid_deriv(d: UvDerivs) -> bool {
    let valid_dudx = d.dudx == d.dudx && abs(d.dudx) < 1000.0;
    let valid_dudy = d.dudy == d.dudy && abs(d.dudy) < 1000.0;
    let valid_dvdx = d.dvdx == d.dvdx && abs(d.dvdx) < 1000.0;
    let valid_dvdy = d.dvdy == d.dvdy && abs(d.dvdy) < 1000.0;
    return valid_dudx && valid_dudy && valid_dvdx && valid_dvdy;
}
```

Use in `compute_texture_lod_from_depth()`:
```wgsl
let d = compute_uv_derivatives_from_depth(...);

if (!is_valid_deriv(d)) {
    return 0.0; // Or visualize the error
}
```

### Step 3: Visualize Invalid Cases

Add debug colors for different error conditions:

```wgsl
// In compute.wgsl after pbr_get_mipmap_levels call
let lod = texture_lods.base_color;

// Debug visualization
if (lod < -10.0) {
    // Red = LOD went negative (derivative explosion)
    color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    return;
}
if (lod > 15.0) {
    // Blue = LOD went too high (derivative collapse)
    color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    return;
}
if (lod != lod) {
    // Magenta = NaN
    color = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    return;
}
```

### Step 4: Check Depth Values

Add visualization for depth:

```wgsl
// In compute.wgsl
let depth = textureLoad(depth_tex, coords, 0);

// Visualize depth
// Near = dark, far = bright
color = vec4<f32>(vec3<f32>(depth), 1.0);
```

If white areas show depth = 1.0, they might be background/skybox that shouldn't be textured.

### Step 5: Check Triangle Data

Verify triangle indices and vertex data are valid:

```wgsl
// In compute.wgsl before calling pbr_get_mipmap_levels
if (triangle_indices.x == triangle_indices.y ||
    triangle_indices.y == triangle_indices.z ||
    triangle_indices.x == triangle_indices.z) {
    // Degenerate triangle - visualize error
    color = vec4<f32>(1.0, 1.0, 0.0, 1.0); // Yellow
    return;
}
```

---

## Test Scenarios

### Test Case 1: Static Camera, Zoom Out
1. Position camera close to PlainGrid.png textured surface
2. Verify lines look crisp (red/yellow LOD)
3. Slowly zoom out
4. **Expected:** Lines gradually blur and disappear (green→cyan→blue LOD)
5. **Bug:** Lines reappear at far distance (stuck at yellow/green LOD)

### Test Case 2: Grazing Angles
1. Look at surface head-on (perpendicular)
2. Verify smooth LOD (no bug)
3. Rotate to grazing angle (nearly parallel to surface)
4. **Expected:** Should still blur at distance
5. **Bug:** Might show artifacts at grazing angles

### Test Case 3: Triangle Edges
1. Zoom out on PlainGrid mesh
2. Look for artifacts at quad edges (where triangles meet)
3. **Expected:** Smooth across entire surface
4. **Bug:** Edges might show different LOD than center

### Test Case 4: White Texture Bug (Bug 2)
1. Load scene that shows white textures
2. Note which models/meshes are affected
3. Try different camera angles
4. Add debug visualization to identify error type
5. **Expected:** All textures show correctly
6. **Bug:** Some areas are white instead of textured

---

## Quick Debug Commands

### Add Debug Visualization to compute.wgsl

**Location:** `compute.wgsl` after line 327

```wgsl
let texture_lods = pbr_get_mipmap_levels(...);

// === DEBUG: Visualize LOD ===
let debug_lod = texture_lods.base_color;
let lod_color = vec3<f32>(
    saturate(debug_lod / 4.0),           // R: 0-4
    saturate((debug_lod - 2.0) / 4.0),   // G: 2-6
    saturate((debug_lod - 4.0) / 4.0)    // B: 4-8
);
textureStore(opaque_tex, coords, vec4<f32>(lod_color, 1.0));
return;
// === END DEBUG ===
```

### Print Debug Values (if available)

If you have printf/debug buffer:
```wgsl
// At far distance where bug occurs
debug_print(lod);
debug_print(rho);
debug_print(dudx_texels);
debug_print(dvdx_texels);
```

---

## Expected vs Actual Behavior

### Expected LOD Progression (zooming out)

| Distance | LOD | Color | Lines Visible |
|----------|-----|-------|---------------|
| Very close | 0-1 | Red | Very crisp |
| Close | 1-2 | Orange | Crisp |
| Medium | 2-4 | Yellow-Green | Slightly blurred |
| Far | 4-6 | Cyan | Blurred, fading |
| Very far | 6-8+ | Blue | Gone/invisible |

### Actual Behavior (Bug)

| Distance | LOD | Color | Lines Visible |
|----------|-----|-------|---------------|
| Very close | 0-1 | Red | Very crisp |
| Close | 1-2 | Orange | Crisp |
| Medium | 2-4 | Yellow-Green | Slightly blurred |
| Far | ??? | ??? | **Multiple lines come back!** |
| Very far | ??? | ??? | ??? |

**Question to answer tomorrow:** What LOD color do you see at far distances when bug occurs?
- If green/yellow: LOD is too low (not blurring enough)
- If blue: LOD is correct, but mipmap generation might be wrong
- If red: LOD calculation is totally broken at far distances

---

## Next Steps for Tomorrow

### For Bug 1 (Multiple Lines at Distance):
1. **Add LOD visualization** (see Step 1 above)
2. **Report LOD color at far distance** when bug occurs
3. **Check for depth discontinuities** (Root Cause #1)
4. **Test with increased LOD bias** (`+1.0` or `+2.0`)
5. **Verify mipmap generation** has all levels
6. **Consider grazing angle correction** if needed

### For Bug 2 (White Textures):
1. **Identify which scenes/models** show white textures
2. **Add `is_valid_lod()` and `is_valid_deriv()` checks**
3. **Add debug visualization** for invalid LOD values (red=negative, blue=too high, magenta=NaN)
4. **Check depth values** in white areas
5. **Add back-face culling check** for UV derivative computation
6. **Test with defensive clamping** on all intermediate values

---

## References

**Standard LOD Formula (OpenGL/DirectX):**
```
rho = max(sqrt(dudx² + dvdx²), sqrt(dudy² + dvdy²))
LOD = log2(rho)
```

**WebGPU Depth Range:** [0, 1] (unlike OpenGL's [-1, 1])

**NDC Conversion:**
```
ndc.x = (pixel.x / screen.x) * 2.0 - 1.0  // [0, width] → [-1, 1]
ndc.y = 1.0 - (pixel.y / screen.y) * 2.0  // [0, height] → [1, -1] (flip Y)
```

---

## File Locations Summary

- **Main LOD calculation:** `crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/helpers/mipmap.wgsl`
- **Call site:** `crates/renderer/src/render_passes/material/opaque/shader/material_opaque_wgsl/compute.wgsl`
- **Mipmap generation:** `crates/renderer-core/src/texture/mipmap.rs`
- **World position reconstruction:** `mipmap.wgsl:reconstruct_world_position()` (lines 56-77)
- **UV derivatives:** `mipmap.wgsl:compute_uv_derivatives_from_depth()` (lines 79-172)
- **LOD computation:** `mipmap.wgsl:compute_texture_lod_from_depth()` (lines 267-310)

---

## Success Criteria

✅ Lines are crisp when close up
✅ Lines gradually blur when zooming out
✅ No triangle seams
✅ Repeating textures work correctly
❌ **Bug 1: Lines should STAY GONE at far distances (not reappear)**
❌ **Bug 2: All textures should display correctly (no white areas)**

---

## Notes

- The depth-based per-pixel approach is fundamentally correct
- This is MUCH better than all previous attempts
- The remaining bug is likely an edge case (depth discontinuities or grazing angles)
- Solution is likely a 5-10 line fix, not a major rewrite

Good luck tomorrow! 🚀
